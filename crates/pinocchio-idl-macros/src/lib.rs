extern crate proc_macro;
use pinocchio_idl_core::{
    DataSlice, Instruction, account_discriminator, classify_seed, count_account_binding,
    find_accounts_param,
};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemFn, ItemStruct, parse_macro_input};

mod helpers;
use helpers::*;

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
                        //let #name = <#ty>::from_le_bytes(data[#start..#end].try_into().unwrap());

                        let #name = <#ty>::from_le_bytes(data.get(#start..#end)
                            .and_then(|s| s.try_into().ok())
                            .ok_or(ProgramError::InvalidArgument)?
                        );
                    });
                }
                Some(DataSlice::Index(idx)) => {
                    injected_statement.push(quote! {
                        //let #name: #ty = data[#idx];
                        let #name: #ty = *data.get(#idx).ok_or(ProgramError::InvalidArgument)?;
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
                    return Err(ProgramError::InvalidAccountData)
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

        if let Some(init) = &account.init {
            if init.0.len() != 2 {
                return syn::Error::new_spanned(
                    &name,
                    "init = [...] requires exactly two expressions: [owner_account, mint_account]",
                )
                .to_compile_error()
                .into();
            }
        }

        if let Some(ata) = &account.ata {
            if ata.0.len() != 2 {
                return syn::Error::new_spanned(
                    &name,
                    "ata = [...] requires exactly two expressions: [owner_account, mint_account]",
                )
                .to_compile_error()
                .into();
            }

            let owner_classes: Vec<_> = match ata
                .0
                .iter()
                .map(|expr| classify_seed(expr, &account_names, &arg_names))
                .collect()
            {
                Ok(c) => c,
                Err(e) => return e.to_compile_error().into(),
            };

            use pinocchio_idl_core::SeedClass;
            let make_address_expr = |class: &SeedClass| -> TokenStream2 {
                match class {
                    SeedClass::Account(ident) => quote! { #ident.address() },
                    SeedClass::Arg(ident) => quote! { ::pinocchio::Address::from(*#ident) },
                    SeedClass::Bytes(bytes) => {
                        let lit = proc_macro2::Literal::byte_string(bytes);
                        quote! { ::pinocchio::Address::from(#lit) }
                    }
                }
            };

            let owner_expr = make_address_expr(&owner_classes[0]);
            let mint_expr = make_address_expr(&owner_classes[1]);

            injected_statement.push(quote! {
                {
                    let __ata_state = ::pinocchio_token::state::Account::from_account_view(#name)?;
                    if __ata_state.owner() != #owner_expr {
                        return Err(ProgramError::IllegalOwner);
                    }
                    if __ata_state.mint() != #mint_expr {
                        return Err(ProgramError::InvalidAccountData);
                    }
                }
            });
        }

        if let Some(pda) = &account.pda {
            let seed_classes: Vec<_> = match pda
                .seeds
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

            use pinocchio_idl_core::PdaProgram;

            let program_expr: TokenStream2 = match &pda.program {
                None => quote! { &crate::ID.to_bytes() },

                Some(PdaProgram::Literal(lit)) => {
                    let s = lit.value();

                    let bytes = pinocchio_idl_core::bs58_decode(s.as_str())
                        .unwrap_or_else(|e| panic!("{e}"));

                    quote! { &[ #(#bytes),* ] }
                }

                Some(PdaProgram::Account(ident)) => {
                    quote! { #ident.address().as_ref() }
                }
            };

            injected_statement.push(quote! {
                {
                    let __expected_pda = ::pinocchio::Address::from(
                        pinocchio_pubkey::derive_address(
                            &[#(#seed_tokens),*],
                            None,
                            #program_expr,
                        )
                    );
                    if #name.address() != &__expected_pda {
                        return Err(ProgramError::InvalidArgument);
                    }
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

    /*
    for (i, statement) in injected_block.stmts.into_iter().enumerate() {
        func.block.stmts.insert(index + i, statement);
    }*/
    let stmts_to_insert = injected_block.stmts.into_iter();
    func.block.stmts.splice(index..index, stmts_to_insert);

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

#[proc_macro_attribute]
pub fn p_state(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_struct = parse_macro_input!(item as ItemStruct);

    /*
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
    */

    let struct_name = &item_struct.ident;
    let discriminator = account_discriminator(&struct_name.to_string());
    let disc_bytes = discriminator.iter().map(|b| quote! { #b });

    TokenStream::from(quote! {
        #[repr(C)]
        #item_struct

        impl #struct_name {
            pub const SPACE: usize = std::mem::size_of::<Self>();
            pub const DISCRIMINATOR: [u8; 8] = [#(#disc_bytes),*];
        }
    })
}

#[proc_macro_attribute]
pub fn p_error(_attr: TokenStream, item: TokenStream) -> TokenStream {
    use syn::ItemEnum;
    let mut item_enum = parse_macro_input!(item as ItemEnum);

    for variant in &mut item_enum.variants {
        variant.attrs.retain(|attr| !attr.path().is_ident("p_code"));
    }

    TokenStream::from(quote! { #item_enum })
}

#[proc_macro_attribute]
pub fn p_constant(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
