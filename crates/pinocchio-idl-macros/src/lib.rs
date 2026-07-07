extern crate proc_macro;
use pinocchio_idl_core::{
    DataSlice, Instruction, account_discriminator, classify_seed, find_accounts_param,
    seed_class_to_tokens,
};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, ExprLit, Fields, Ident, ItemFn, ItemStruct, Lit, Pat, Stmt, Type, parse_macro_input,
};

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

    let required = parsed_attr.accounts.len();

    let account_names: Vec<String> = parsed_attr
        .accounts
        .iter()
        .map(|a| a.name.to_string())
        .collect();

    let arg_names: Vec<String> = parsed_attr
        .data
        .as_ref()
        .map(|fields| fields.iter().map(|f| f.name.to_string()).collect())
        .unwrap_or_default();

    let bump_field: Option<Ident> = parsed_attr
        .data
        .as_ref()
        .and_then(|fields| fields.iter().find(|d| d.name == "bump"))
        .map(|d| d.name.clone());

    //if parsed_attr.data.is_some() {
    if let Some(data) = &parsed_attr.data {
        for data_args in data {
            let name = &data_args.name;
            let ty = &data_args.ty;

            match &data_args.slice {
                Some(DataSlice::Range(range)) => {
                    let start = &range.start;
                    let end = &range.end;
                    injected_statement.push(quote! {
                        let #name = <#ty>::from_le_bytes(data[#start..#end].try_into().unwrap());
                    });
                }
                Some(DataSlice::Index(idx)) => {
                    injected_statement.push(quote! {
                        let #name: #ty = data[#idx];
                    });
                }
                None => {
                    injected_statement.push(quote! {
                        let #name: #ty;
                    });
                }
            }
        }
    }

    for account in parsed_attr.accounts {
        let name = account.name;

        if account.is_mut {
            injected_statement.push(quote! {
                if !#name.is_writable() {
                    return Err(ProgramError::MissingRequiredSignature)
                }
            });
        }

        if account.is_signer {
            injected_statement.push(quote! {
                if !#name.is_signer() {
                    return Err(ProgramError::MissingRequiredSignature)
                }
            });
        }

        /* Disabled for now */
        if let Some(pda_seeds) = &account.pda_seeds {
            let seed_classes: Vec<_> = match pda_seeds
                .0
                .iter()
                .map(|expr| classify_seed(expr, &account_names, &arg_names))
                .collect()
            {
                Ok(c) => c,
                Err(e) => return e.to_compile_error().into(),
            };

            let seed_tokens: Vec<TokenStream2> = match seed_classes
                .iter()
                .map(|class| {
                    seed_class_to_tokens(class, parsed_attr.data.as_deref().unwrap_or(&[]))
                })
                .collect()
            {
                Ok(t) => t,
                Err(e) => return e.to_compile_error().into(),
            };

            let bump_ident =
                match &bump_field {
                    Some(ident) => ident,
                    None => {
                        return syn::Error::new_spanned(
                        &name,
                        "pda verification requires a `bump: u8 = data[N]` field in `data = [...]`",
                    ).to_compile_error().into();
                    }
                };

            injected_statement.push(quote! {

                let pda_seeds: &[&[u8]] = &[#(#seed_tokens),*];

                let expected_pda = ::pinocchio::Address::from(pinocchio_pubkey::derive_address(
                    &[#(#seed_tokens),*, &[#bump_ident]],
                    None,
                    &crate::ID.to_bytes(),
                ));

                if #name.address() != &expected_pda {
                    return Err(ProgramError::InvalidArgument);
                }
            });
        }
    }

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

    let bounds_check: syn::Stmt = syn::parse_quote! {
        if #account_ident.len() < #required {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
    };
    func.block.stmts.insert(0, bounds_check);

    TokenStream::from(quote! {
        #func
    })
}

fn strip(mut expr: &Expr) -> &Expr {
    loop {
        expr = match expr {
            Expr::Reference(r) => &r.expr,
            Expr::Paren(p) => &p.expr,
            Expr::Group(g) => &g.expr,
            _ => return expr,
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
            if let Pat::Slice(_slice) = &local.pat {
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

#[proc_macro_attribute]
pub fn p_state(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_struct = parse_macro_input!(item as ItemStruct);

    let fields = match &item_struct.fields {
        Fields::Named(named) => &named.named,
        _ => {
            return syn::Error::new_spanned(
                &item_struct,
                "#[p_state] requires a struct with named fields",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut space: usize = 0;
    for field in fields {
        space += match field_byte_size(&field.ty) {
            Ok(s) => s,
            Err(e) => return e.to_compile_error().into(),
        };
    }

    let struct_name = &item_struct.ident;
    let discriminator = account_discriminator(&struct_name.to_string());
    let disc_bytes = discriminator.iter().map(|b| quote! { #b });

    TokenStream::from(quote! {
        #item_struct

        impl #struct_name {
            pub const SPACE: usize = #space;
            pub const DISCRIMINATOR: [u8; 8] = [#(#disc_bytes),*];
        }
    })
}

fn field_byte_size(ty: &Type) -> syn::Result<usize> {
    match ty {
        Type::Path(p) => {
            let ident = p
                .path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(p, "empty type path"))?
                .ident
                .to_string();
            match ident.as_str() {
                "u8" | "i8" | "bool" => Ok(1),
                "u16" | "i16" => Ok(2),
                "u32" | "i32" => Ok(4),
                "u64" | "i64" => Ok(8),
                "u128" | "i128" => Ok(16),
                "Pubkey" | "Address" => Ok(32),
                other => Err(syn::Error::new_spanned(
                    p,
                    format!(
                        "#[p_state] doesn't know the size of `{other}` — \
                                use an integer, bool, Pubkey, Address, or fixed-size array"
                    ),
                )),
            }
        }
        Type::Array(arr) => {
            let elem_size = field_byte_size(&arr.elem)?;
            let len: usize = match &arr.len {
                Expr::Lit(ExprLit {
                    lit: Lit::Int(n), ..
                }) => n.base10_parse()?,
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "array length must be an integer literal",
                    ));
                }
            };
            Ok(elem_size * len)
        }
        other => Err(syn::Error::new_spanned(
            other,
            "unsupported field type for #[p_state]",
        )),
    }
}
