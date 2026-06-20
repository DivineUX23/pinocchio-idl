extern crate proc_macro;
//use pinocchio::account;
use proc_macro::TokenStream;
use quote::quote;
use pinocchio_idl_core::{Instruction, find_accounts_param};
use syn::{Expr, ItemFn, Pat, Stmt, parse_macro_input};

/*
pub mod program_error {
    #[derive(Debug, PartialEq, Eq)]
    pub enum ProgramError {
        NotEnoughAccountKeys,
        MissingRequiredSignature,
        InvalidArgument,
    }
}*/


#[proc_macro_attribute]
pub fn p_instruction(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_attr = parse_macro_input!(attr as Instruction);

    let mut func = parse_macro_input!(item as ItemFn);

    let mut injected_statement = Vec::new();



    //if parsed_attr.data.is_some() {
    if let Some(data) = &parsed_attr.data {
        
        for data_args in data {
            let name = &data_args.name;
            let ty = &data_args.ty;

            if let Some(slice) = &data_args.slice {

                let start = &slice.start;
                let end = &slice.end;

                injected_statement.push(quote!{
                    let #name = <#ty>::from_le_bytes(data[#start..#end].try_into().unwrap());
                    }
                );
            } else {

                injected_statement.push(quote!{
                    let #name = #ty;
                    }
                )
            }
        }

    };


    for account in parsed_attr.accounts {
        let name = account.name;
        if account.is_mut {
            injected_statement.push(quote!{
                if !#name.is_writable() {
                    return Err(pinocchio::ProgramError::MissingRequiredSignature)
                }
            });
        }

        if account.is_signer {
            injected_statement.push(quote!{
                if !#name.is_signer() {
                    return Err(pinocchio::ProgramError::MissingRequiredSignature)
                }
            });
        }

        /* */
        if let Some(pda_seeds) = account.pda_seeds {
            injected_statement.push(quote!{
                let (expected_pda, _bump) = pinocchio::pubkey::Pubkey::find_program_address(
                    &[#pda_seeds],
                    program_id
                );
                if #name.key() != expected_pda {
                    return Err(pinocchio::ProgramError::InvalidArgument)
                }
            });
        }
    }

    /*
    for statement in injected_statement.into_iter() {
        let element = syn::parse_quote!(#statement);
        func.block.stmts.insert(0, element);
    }
    */

    let all_injections = quote! {
        #(#injected_statement)*
    };

    let injected_block: syn::Block = syn::parse_quote!({
        #all_injections
    });


    /*
    let mut index = 0;

    for (i, statement) in func.block.stmts.iter().enumerate() {
        if let syn::Stmt::Local(_) = statement {
            index = i + 1;
        } else {
            break;
        }
    }
    */

    let account_ident = match find_accounts_param(&func.sig) {
        Ok(ident) => ident,
        Err(err) => return err.to_compile_error().into(),
    };

    let accounts_name = account_ident.to_string();
    let index = count_account_binding(&func.block.stmts, &accounts_name);


    for (i, statement) in injected_block.stmts.into_iter().enumerate() {
        func.block.stmts.insert(index + i, statement);
    }

    TokenStream::from(quote!{
        #func
    })

}


fn strip(mut expr: &Expr) -> &Expr {
    loop {
        expr = match expr {
            Expr::Reference(r) => &r.expr,
            Expr::Paren(p) => &p.expr,
            Expr::Group(g) => &g.expr,
            _ => return expr
        }
    }
}

fn is_path_ident(expr: &Expr, name: &str) -> bool {
    matches!(strip(expr), Expr::Path(p) if p.path.is_ident(name))
}

fn is_indexed_account(expr: &Expr, name: &str) -> bool {
    match strip(expr) {
        Expr::Index(idx) => is_path_ident(&idx.expr, name),
        _ => false,
    }
}
fn count_account_binding(stmts: &[Stmt], accounts_param: &str) -> usize {
    let mut index = 0;

    for (i, stmt) in stmts.iter().enumerate() {
        if let Stmt::Local(local) = stmt {
            if let Pat::Slice(slice) = &local.pat {
                if let Some(init) = &local.init {
                    if is_path_ident(&init.expr, accounts_param) {
                        //return slice.elems.iter().filter(|p| !matches!(p, Pat::Rest(_))).count();
                        return i + 1;
                    }
                }
            }

            if let Some(init) = &local.init {
                if is_indexed_account(&init.expr, accounts_param) {
                    index = i + 1;
                    continue;
                }
            } 
        } else {
            break;
        }
        /* 
        stmts.iter().take_while(
            |stmt| {
                matches!(stmt, Stmt::Local(local)
                    if local.init.as_ref()
                        .is_some_and(|i| is_indexed_account(&i.expr, accounts_param)))
            })
            .count()
        */
    }
    index
}

//fn is_