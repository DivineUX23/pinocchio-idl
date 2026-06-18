extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use pinocchio_idl_core::Instruction;
use syn::{parse_macro_input, ItemFn};

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


    for statement in injected_statement.into_iter() {
        let element = syn::parse_quote!(#statement);
        func.block.stmts.insert(0, element);
    }

    TokenStream::from(quote!{
        #func
    })

}
