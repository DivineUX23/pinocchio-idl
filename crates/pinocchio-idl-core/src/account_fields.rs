use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{
    Expr, Ident, Token,
    parse::{Parse, ParseStream},
};

use crate::{IdlPda, IdlPdaProgram, IdlPdaSeed, bs58_decode, seed_expr_to_idl};

#[derive(Debug, Clone)]
pub enum PdaProgram {
    Literal(syn::LitStr),
    Account(Ident),
}

#[derive(Debug, Clone)]
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
        if self.0.len() != 2 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "ata = [...] requires exactly two expressions: [owner, mint]",
            ));
        }

        let owner_seed = seed_expr_to_idl(&self.0[0], account_names, arg_names)?;
        let mint_seed = seed_expr_to_idl(&self.0[1], account_names, arg_names)?;

        // Canonical Associated Token Program: ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
        const ATOKEN_ID: [u8; 32] = [
            140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19,
            153, 218, 255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
        ];

        // Canonical SPL Token Program: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        const TOKEN_PROGRAM_ID: [u8; 32] = [
            6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180,
            133, 237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
        ];

        let token_program_seed = IdlPdaSeed::Const {
            value: TOKEN_PROGRAM_ID.to_vec(),
        };

        // Canonical ATA seeds: [wallet, token_program, mint]
        let seeds = vec![owner_seed, token_program_seed, mint_seed];

        Ok(IdlPda {
            seeds,
            program: Some(IdlPdaProgram::Const {
                value: ATOKEN_ID.to_vec(),
            }),
        })
    }
}

#[derive(Debug, Clone)]
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
                } else {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "Unknown named parameter `{}` in PDA seeds. Only `program = ...` is supported.",
                            key
                        ),
                    ));
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
        let mut seeds = self
            .seeds
            .iter()
            .map(|expr| seed_expr_to_idl(expr, account_names, arg_names))
            .collect::<syn::Result<Vec<_>>>()?;

        // If the trailing seed is an argument named "bump" (e.g. bump, bump_seed, bumpSeed),
        // it represents the canonical PDA derivation bump passed to on-chain `derive_address`.
        // In client-facing IDLs (Anchor & Codama), seeds represent the input to `findProgramAddress`
        // which searches for the bump. Including the bump causes client helpers to search for
        // a second bump, deriving an invalid address.
        if let Some(last) = seeds.last() {
            if match last {
                IdlPdaSeed::Arg { path } => {
                    let p = path.to_ascii_lowercase();
                    p == "bump" || p == "bump_seed" || p == "bumpseed"
                }
                _ => false,
            } {
                seeds.pop();
            }
        }

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
