use syn::{
    Ident, LitInt, Stmt, Token, bracketed,
    parse::{Parse, ParseStream},
};

pub mod account_fields;
pub mod cli_struct;
pub mod helpers;

pub use account_fields::*;
pub use cli_struct::*;
pub use helpers::*;

#[derive(Debug)]
pub struct Instruction {
    pub id: Option<u8>,
    pub accounts: Vec<Account>,
    pub data: Option<Vec<Data>>,
}

impl Parse for Instruction {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut accounts = Vec::new();
        let mut data = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            if ident == "id" {
                input.parse::<Token![=]>()?;
                let lit: LitInt = input.parse()?;
                id = Some(lit.base10_parse()?);
            } else if ident == "accounts" {
                input.parse::<Token![=]>()?;
                let content;
                bracketed!(content in input);

                let parsed = content.parse_terminated(Account::parse, Token![,])?;

                accounts = parsed.into_iter().collect();
            } else if ident == "data" {
                input.parse::<Token![=]>()?;
                let content;
                bracketed!(content in input);

                let parsed = content.parse_terminated(Data::parse, Token![,])?;

                let raw_data = parsed.into_iter().collect();
                data = Some(raw_data)
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("unknown key `{}` in #[p_instruction(...)]", ident),
                ));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        /*
        match ident.to_string().as_str() {
            "id" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitInt = input.parse()?;
                    id = Some(lit.base10_parse()?);
                }

                "accounts" => {
                    input.parse::<Token![=]>()?;
                    let content;
                    bracketed!(content in input);

                    let parsed = content.parse_terminated(Account::parse, Token![,])?;

                    accounts = parsed.into_iter().collect();
                }

                "data" => {
                    input.parse::<Token![=]>()?;
                    let content;
                    bracketed!(content in input);

                    let parsed = content.parse_terminated(Data::parse, Token![,])?;

                    let raw_data = parsed.into_iter().collect();
                    data = Some(raw_data)
                }

                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown key `{other}` in #[p_instruction(...)]"),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        */

        Ok(Instruction { id, accounts, data })
    }
}

impl Instruction {
    pub fn add_accounts(&mut self, stmts: &[Stmt], accounts_param: &str) {
        let binded_accounts = account_binding(stmts, accounts_param);

        let new_binds: Vec<Ident> = binded_accounts
            .into_iter()
            .filter(|bind| !self.accounts.iter().any(|acc| acc.name == *bind))
            .collect();

        let other_accounts: Vec<Account> =
            new_binds.into_iter().map(Account::new_from_ident).collect();

        self.accounts.extend(other_accounts);

        //Instruction
    }

    pub fn into_idl(&self, name: String, index: u8) -> syn::Result<IdlInstruction> {
        let account_names: Vec<String> = self.accounts.iter().map(|a| a.name.to_string()).collect();

        let arg_names: Vec<String> = self
            .data
            .as_ref()
            .map(|fields| fields.iter().map(|f| f.name.to_string()).collect())
            .unwrap_or_default();

        let accounts = self
            .accounts
            .iter()
            .map(|acc| acc.into_idl(&account_names, &arg_names))
            .collect::<syn::Result<Vec<_>>>()?;

        let args = self
            .data
            .as_ref()
            .map(|fields| {
                fields
                    .iter()
                    .map(Data::into_idl_arg)
                    .collect::<syn::Result<Vec<_>>>()
            })
            .transpose()?;

        Ok(IdlInstruction {
            name,
            discriminator: vec![self.id.unwrap_or(index)],
            accounts,
            args,
        })
    }
}

#[derive(Debug)]
pub struct Account {
    pub name: Ident,
    pub is_signer: bool,
    pub is_mut: bool,
    pub pda: Option<Seed>,
    pub ata: Option<Ata>,
    pub init: Option<Ata>,
    pub struct_state: Option<Ident>,
    pub address: Option<syn::LitStr>,
    pub relations: Vec<Ident>,
}

impl Parse for Account {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let mut is_signer = false;
        let mut is_mut = false;
        let mut pda = None;
        let mut ata = None;
        let mut init = None;
        let mut struct_state = None;
        let mut address = None;
        let mut relations = Vec::new();

        if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);

            while !content.is_empty() {
                if content.peek(Token![mut]) {
                    content.parse::<Token![mut]>()?;
                    is_mut = true;
                    if content.peek(Token![,]) {
                        content.parse::<Token![,]>()?;
                    }
                    continue;
                }

                let ident: Ident = content.parse()?;

                if ident == "signer" {
                    is_signer = true;
                } else if ident == "pda" {
                    content.parse::<Token![=]>()?;
                    let seeds: Seed = content.parse()?;

                    pda = Some(seeds);
                } else if ident == "ata" {
                    content.parse::<Token![=]>()?;
                    let atas: Ata = content.parse()?;

                    ata = Some(atas);
                } else if ident == "init" {
                    content.parse::<Token![=]>()?;
                    let inits: Ata = content.parse()?;

                    init = Some(inits);
                } else if ident == "state" {
                    content.parse::<Token![=]>()?;
                    struct_state = Some(content.parse::<Ident>()?);
                } else if ident == "address" {
                    content.parse::<Token![=]>()?;
                    address = Some(content.parse::<syn::LitStr>()?);
                } else if ident == "relations" {
                    content.parse::<Token![=]>()?;
                    let inner;

                    syn::bracketed!(inner in content);

                    let idents = inner.parse_terminated(Ident::parse, Token![,])?;
                    relations = idents.into_iter().collect();
                } else {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown account constraint `{}`, accepted constraint are - `signer`, `mut`, `pda`, `ata`, `init`, `state`, `address`, `relations`",
                            ident
                        ),
                    ));
                }

                /*
                match ident.to_string().as_str() {
                    "signer" => is_signer = true,
                    "pda" => {
                        content.parse::<Token![=]>()?;
                        let seeds: Seed = content.parse()?;
                        pda = Some(seeds);
                    }

                    "ata" => {
                        content.parse::<Token![=]>()?;
                        let atas: Ata = content.parse()?;
                        ata = Some(atas);
                    }

                    "state" => {
                        content.parse::<Token![=]>()?;
                        struct_state = Some(content.parse::<Ident>()?);
                    }
                    "address" => {
                        content.parse::<Token![=]>()?;
                        address = Some(content.parse::<syn::LitStr>()?);
                    }
                    "relations" => {
                        content.parse::<Token![=]>()?;
                        let inner;
                        syn::bracketed!(inner in content);
                        let idents = inner.parse_terminated(Ident::parse, Token![,])?;
                        relations = idents.into_iter().collect();
                    }

                    other => {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!("unknown account constraint `{other}`"),
                        ));
                    }
                }
                */

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
            /*
            if pda.is_some() && ata.is_some() {
                return Err(syn::Error::new(
                    name.span(),
                    "Account cannot be both a PDA and an ATA simultaneously.",
                ));
            }
            */
        }

        Ok(Account {
            name,
            is_signer,
            is_mut,
            pda,
            ata,
            init,
            struct_state,
            address,
            relations,
        })
    }
}

impl Account {
    pub fn new_from_ident(name: Ident) -> Account {
        Account {
            name,
            is_signer: false,
            is_mut: false,
            pda: None,
            ata: None,
            init: None,
            struct_state: None,
            address: None,
            relations: Vec::new(),
        }
    }

    pub fn into_idl(
        &self,
        account_names: &[String],
        arg_names: &[String],
    ) -> syn::Result<IdlAccount> {
        let mut pda_data = None;

        if self.pda.is_some() {
            pda_data = self
                .pda
                .as_ref()
                .map(|seed| seed.into_idl(account_names, arg_names))
                .transpose()?;
        } else if self.ata.is_some() {
            pda_data = self
                .ata
                .as_ref()
                .map(|ata| ata.into_idl(account_names, arg_names))
                .transpose()?;
        } else if self.init.is_some() {
            pda_data = self
                .init
                .as_ref()
                .map(|ata| ata.into_idl(account_names, arg_names))
                .transpose()?;
        }

        let mut address_val = self.address.as_ref().map(|lit| lit.value());
        let raw_name = self.name.to_string().trim_start_matches('_').to_string();
        let mut idl_name = raw_name.clone();

        let lookup_key = address_val.as_deref().unwrap_or(raw_name.as_str());

        if let Some((canonical_name, canonical_address)) = known_program(lookup_key) {
            idl_name = canonical_name.to_string();
            address_val = Some(canonical_address.to_string());
        }

        Ok(IdlAccount {
            name: idl_name,
            writable: self.is_mut,
            signer: self.is_signer,
            address: address_val,
            relations: (!self.relations.is_empty())
                .then(|| self.relations.iter().map(|r| r.to_string()).collect()),

            pda: pda_data,
            state: self.struct_state.as_ref().map(|s| s.to_string()),
        })
    }
}

pub fn known_program(name_or_address: &str) -> Option<(&'static str, &'static str)> {
    match name_or_address {
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" | "token_program" | "tokenProgram" => Some((
            "token_program",
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        )),
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        | "token_2022_program"
        | "token2022Program" => Some((
            "token_2022_program",
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        )),
        "11111111111111111111111111111111" | "system_program" | "systemProgram" => {
            Some(("system_program", "11111111111111111111111111111111"))
        }
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        | "associated_token_program"
        | "associatedTokenProgram" => Some((
            "associated_token_program",
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
        )),
        "SysvarRent111111111111111111111111111111111" | "rent" | "sysvarRent" | "rent_sysvar" => {
            Some(("rent_sysvar", "SysvarRent111111111111111111111111111111111"))
        }
        "SysvarC1ock11111111111111111111111111111111"
        | "clock"
        | "sysvarClock"
        | "clock_sysvar" => Some((
            "clock_sysvar",
            "SysvarC1ock11111111111111111111111111111111",
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let s = r#"
            id = 1,
            accounts = [
                maker(signer),
                escrow(mut, pda=["escrow", maker, seed], state=EscrowState)
            ],
            data = [
                seed: u64 = data[0..8],
                amount: u64 = data[8..16]
            ]
        "#;

        let parsed: Instruction = syn::parse_str(s).expect("failed to parse instruction");
        println!("{:#?}", parsed);
    }
}
