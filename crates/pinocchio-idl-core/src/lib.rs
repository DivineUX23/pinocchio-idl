/*
#[p_instruction(
    accounts = [
        maker(signer),
        escrow(mut, pda=["escrow", maker, seed], state=EscrowState)
    ]
    data = [
       seed: u64 = data[0..8],
       amount: u64 = data[8..16]
    ]
)]
    user ( mut , signer , pda = [b"user", user_key.as_ref()] , state = UserState )
*/

use proc_macro2::{TokenStream as TokenStream2};
use quote::ToTokens;
use syn::{Expr, FnArg, Ident, Pat, Stmt, Signature, LitInt, Token, bracketed, parse::{Parse, ParseStream}, Type};

#[derive(Debug)]
pub struct Instruction {
    pub id: Option<u8>,
    pub accounts: Vec<Account>,
    pub data: Option<Vec<Data>>
}

impl Parse for Instruction {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut accounts = Vec::new();
        let mut data = None;


        while !input.is_empty() {

            let ident: Ident = input.parse()?;

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
                    return Err(syn::Error::new (
                        ident.span(),
                        format!("unknown key `{other}` in #[p_instruction(...)]")
                    ));
                }

            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }

        }

        Ok(Instruction { id, accounts, data })

    }
}

impl Instruction {
    pub fn add_accounts(&mut self, stmts: &[Stmt], accounts_param: &str) {

        let binded_accounts = count_account_binding(stmts, accounts_param);

        let new_binds: Vec<Ident> = binded_accounts
            .into_iter()
            .filter(|bind| !self.accounts.iter().any(|acc| acc.name == *bind))
            .collect();

        let other_accounts: Vec<Account> = new_binds
            .into_iter()
            .map(Account::new_from_ident).collect();

        self.accounts.extend(other_accounts);

        //Instruction

    }
}


#[derive(Debug)]
pub struct Account {
    pub name: Ident,
    pub is_signer: bool,
    pub is_mut: bool,
    pub pda_seeds: Option<Seed>,
    pub struct_state: Option<Ident>,
}

impl Parse for Account {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let mut is_signer = false;
        let mut is_mut = false;
        let mut pda_seeds = None;
        let mut struct_state = None;

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
                match ident.to_string().as_str() {
                    "signer" => is_signer = true,
                    "pda" => {
                        content.parse::<Token![=]>()?;
                        let seeds: Seed = content.parse()?;
                        pda_seeds = Some(seeds);
                    }

                    "state" => {
                        content.parse::<Token![=]>()?;
                        struct_state = Some(content.parse::<Ident>()?);
                    }
                    other => {
                        return Err(syn::Error::new (
                            ident.span(),
                            format!("unknown account constraint `{other}`")
                        ))
                    }
                }

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }

            }


        }

        Ok(Account { name, is_signer, is_mut, pda_seeds, struct_state })

    }
}

impl Account {
    pub fn new_from_ident(name: Ident) -> Account {
        Account {
            name,
            is_signer: false,
            is_mut: false,
            pda_seeds: None,
            struct_state: None,
        }
    }
}




#[derive(Debug)]
pub struct Seed(Vec<Expr>);

impl Parse for Seed {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        //let _: Token![pda] = input.parse()?;
        //let _: Token![=] = input.parse()?;
        let content;
        syn::bracketed!(content in input);
        let exprs = content.parse_terminated(Expr::parse, Token![,])?;
        Ok(
            Seed(exprs.into_iter().collect())
        )
    }
}

impl ToTokens for Seed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {

        for (i, expr) in self.0.iter().enumerate() {
            if i > 0 {
                Token![,](proc_macro2::Span::call_site()).to_tokens(tokens);
            }
            expr.to_tokens(tokens);
        }

    }
}


#[derive(Debug)]
pub struct Data {
    pub name: Ident,
    pub ty: Type,
    //pub slice_start: Option<usize>,
    //pub slice_end: Option<usize>

    pub slice: Option<syn::ExprRange>
}


impl Parse for Data {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        //let mut slice_start = None;
        //let mut slice_end = None;

        input.parse::<Token![:]>()?;
        let ty = input.parse()?;

        let mut slice = None;

        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let ident: Ident = input.parse()?;

            if ident.to_string().as_str() == "data" {
                let content;
                syn::bracketed!( content in input);
                let range: syn::ExprRange = content.parse()?;
                slice = Some(range);

                //slice_start = Some(content.parse());
                //content.parse::<Token![..]>()?;
                //slice_end = Some(content.parse());
            } else {
                return Err(syn::Error::new(ident.span(), "expected `data`"));
            }

        }

        //Ok(Data { name, ty, slice_start, slice_end })
        Ok(Data { name, ty, slice })

    }
}


pub struct State {
    pub name: Ident,
    pub fields: Vec<Fields>
}

pub struct Fields {
    pub name: Ident,
    pub ty: Type
}




///Helper:

pub fn find_accounts_param(sig: &Signature) -> syn::Result<Ident> {
    for arg in &sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                if pat_ident.ident == "accounts" {
                    return Ok(pat_ident.ident.clone());
                }
            }
        }
     }

    Err(syn::Error::new_spanned(
        &sig.ident, 
        format!(
            "`{}` must take a parameter literally named `accounts` \
             (e.g. `accounts: &[AccountView]`) — account bindings are located \
             by this exact name, regardless of position or type",
            sig.ident
        ),
    ))
}


/// Accounts.Parse the list of binded accounts and scan their results to ensure they don't... 
/// already appear in the list of instruction.acconts the add them to ths list of instruction.accounts

pub fn add_accounts(stmts: &[Stmt], accounts_param: &str, instruction: &mut Instruction) {

    let binded_accounts = count_account_binding(stmts, accounts_param);

    let new_binds: Vec<Ident> = binded_accounts
        .into_iter()
        .filter(|bind| !instruction.accounts.iter().any(|acc| acc.name == *bind))
        .collect();

    let other_accounts: Vec<Account> = new_binds
        .into_iter()
        .map(|bind| Account {
            name: bind,
            is_signer: false,
            is_mut: false,
            pda_seeds: None,
            struct_state: None,
    }).collect();

    instruction.accounts.extend(other_accounts);

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

pub fn count_account_binding(stmts: &[Stmt], accounts_param: &str) -> Vec<Ident> {
    let mut binding = Vec::new();

    for stmt in stmts {
        if let Stmt::Local(local) = stmt {
            if let Pat::Slice(slice) = &local.pat {
                if let Some(init) = &local.init {
                    if is_path_ident(&init.expr, accounts_param) {
                        //let binding = slice.elems.iter().filter(|p| !matches!(p, Pat::Rest(_))).collect();
                        for pat in slice.elems.iter() {
                            if let Pat::Ident(pat_ident) = pat {
                                binding.push(pat_ident.ident.clone());
                            }
                        }
                    }
                }
            }

            if let Some(init) = &local.init {
                if is_indexed_account(&init.expr, accounts_param) {

                    if let Pat::Ident(pat_ident) = &local.pat {
                        binding.push(pat_ident.ident.clone());
                    }

                    continue;
                }
            } 
        } else {
            break;
        }
    }
    binding
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
