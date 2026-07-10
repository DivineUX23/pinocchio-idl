use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{
    Expr, Ident, Token,
    parse::{Parse, ParseStream},
};

use crate::{IdlPda, IdlPdaProgram, bs58_decode, seed_expr_to_idl};

#[derive(Debug, Clone)]
pub enum PdaProgram {
    Literal(syn::LitStr),
    Account(Ident),
}

#[derive(Debug)]
pub struct Ata(pub Vec<Expr>);

impl Parse for Ata {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);
        let exprs = content.parse_terminated(Expr::parse, Token![,])?;
        Ok(Ata(exprs.into_iter().collect()))
    }
}

impl ToTokens for Ata {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for (i, expr) in self.0.iter().enumerate() {
            if i > 0 {
                Token![,](proc_macro2::Span::call_site()).to_tokens(tokens);
            }
            expr.to_tokens(tokens);
        }
    }
}

impl Ata {
    pub fn into_idl(&self, account_names: &[String], arg_names: &[String]) -> syn::Result<IdlPda> {
        let seeds = self
            .0
            .iter()
            .map(|expr| seed_expr_to_idl(expr, account_names, arg_names))
            .collect::<syn::Result<Vec<_>>>()?;

        // associated Token program address
        let atoken_bytes: Vec<u8> = bs58_decode("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
            .expect("AToken address is valid Base58");

        Ok(IdlPda {
            seeds,
            program: Some(IdlPdaProgram::Const {
                value: atoken_bytes,
            }),
        })
    }
}

#[derive(Debug)]
pub struct Seed {
    pub seeds: Vec<Expr>,
    pub program: Option<PdaProgram>,
}

impl Parse for Seed {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);

        let mut seeds = Vec::new();
        let mut program: Option<PdaProgram> = None;

        while !content.is_empty() {
            if content.peek(Ident) && content.peek2(Token![=]) {
                let key: Ident = content.parse()?;

                if key == "program" {
                    content.parse::<Token![=]>()?;

                    if content.peek(syn::LitStr) {
                        program = Some(PdaProgram::Literal(content.parse()?));
                    } else {
                        program = Some(PdaProgram::Account(content.parse()?));
                    }

                    if content.peek(Token![,]) {
                        content.parse::<Token![,]>()?;
                    }
                    continue;
                }
            }

            let expr: Expr = content.parse()?;
            seeds.push(expr);

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Seed { seeds, program })
    }
}

impl ToTokens for Seed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        for (i, expr) in self.seeds.iter().enumerate() {
            if i > 0 {
                Token![,](proc_macro2::Span::call_site()).to_tokens(tokens);
            }
            expr.to_tokens(tokens);
        }
    }
}

impl Seed {
    pub fn into_idl(&self, account_names: &[String], arg_names: &[String]) -> syn::Result<IdlPda> {
        let seeds = self
            .seeds
            .iter()
            .map(|expr| seed_expr_to_idl(expr, account_names, arg_names))
            .collect::<syn::Result<Vec<_>>>()?;

        let program = match &self.program {
            None => None,

            Some(PdaProgram::Literal(lit)) => {
                let bytes =
                    bs58_decode(&lit.value()).map_err(|e| syn::Error::new(lit.span(), e))?;

                Some(IdlPdaProgram::Const { value: bytes })
            }

            Some(PdaProgram::Account(ident)) => Some(IdlPdaProgram::Account {
                path: ident.to_string(),
            }),
        };

        Ok(IdlPda { seeds, program })
    }
}
