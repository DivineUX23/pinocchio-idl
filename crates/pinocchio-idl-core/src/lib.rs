use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::quote;
use syn::{
    Expr, FnArg, Ident, Lit, LitInt, Pat, Signature, Stmt, Token, Type, bracketed,
    parse::{Parse, ParseStream},
};

pub mod cli_struct;
pub use cli_struct::*;

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
    pub pda_seeds: Option<Seed>,
    pub struct_state: Option<Ident>,
    pub address: Option<syn::LitStr>,
    pub relations: Vec<Ident>,
}

impl Parse for Account {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let mut is_signer = false;
        let mut is_mut = false;
        let mut pda_seeds = None;
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

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
        }

        Ok(Account {
            name,
            is_signer,
            is_mut,
            pda_seeds,
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
            pda_seeds: None,
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
        let pda_seeds = self
            .pda_seeds
            .as_ref()
            .map(|seed| seed.into_idl(account_names, arg_names))
            .transpose()?;

        Ok(IdlAccount {
            name: self.name.to_string().trim_start_matches('_').to_string(),
            writable: self.is_mut,
            signer: self.is_signer,
            address: self.address.as_ref().map(|lit| lit.value()),
            relations: (!self.relations.is_empty())
                .then(|| self.relations.iter().map(|r| r.to_string()).collect()),
            pda_seeds,
            state: self.struct_state.as_ref().map(|s| s.to_string()),
        })
    }
}

#[derive(Debug)]
pub struct Seed(pub Vec<Expr>);

impl Parse for Seed {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        //let _: Token![pda] = input.parse()?;
        //let _: Token![=] = input.parse()?;
        let content;
        syn::bracketed!(content in input);
        let exprs = content.parse_terminated(Expr::parse, Token![,])?;
        Ok(Seed(exprs.into_iter().collect()))
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

impl Seed {
    pub fn into_idl(&self, account_names: &[String], arg_names: &[String]) -> syn::Result<IdlPda> {
        let seeds = self
            .0
            .iter()
            .map(|expr| seed_expr_to_idl(expr, account_names, arg_names))
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(IdlPda {
            seeds,
            program: None,
        })
    }
}

#[derive(Debug)]
pub enum SeedClass {
    Bytes(Vec<u8>),
    Account(Ident),
    Arg(Ident),
}

pub fn classify_seed(
    expr: &Expr,
    account_names: &[String],
    arg_names: &[String],
) -> syn::Result<SeedClass> {
    match strip(expr) {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Str(s) => Ok(SeedClass::Bytes(s.value().into_bytes())),
            Lit::ByteStr(b) => Ok(SeedClass::Bytes(b.value())),

            other => Err(syn::Error::new_spanned(
                other,
                "pda seed literal must be a string or byte string",
            )),
        },

        Expr::Path(p) => {
            let ident = p.path.get_ident().ok_or_else(|| {
                syn::Error::new_spanned(p, "pda seed path must be a single identifier")
            })?;

            let name = ident.to_string();

            if account_names.iter().any(|a| a == &name) {
                Ok(SeedClass::Account(ident.clone()))
            } else if arg_names.iter().any(|a| a == &name) {
                Ok(SeedClass::Arg(ident.clone()))
            } else {
                Err(syn::Error::new_spanned(
                    p,
                    format!("seed `{name}` doesn't match any declared account or data field"),
                ))
            }
        }
        other => Err(syn::Error::new_spanned(
            other,
            "pda seed must be a string/byte-string literal or an identifier",
        )),
    }
}

#[derive(Debug)]
pub enum DataSlice {
    Range(syn::ExprRange),
    Index(syn::Expr),
}

#[derive(Debug)]
pub struct Data {
    pub name: Ident,
    pub ty: Type,
    //pub slice_start: Option<usize>,
    //pub slice_end: Option<usize>
    pub slice: Option<DataSlice>,
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
                //let range: syn::ExprRange = content.parse()?;
                //slice = Some(range);

                slice = Some(if content.fork().parse::<syn::ExprRange>().is_ok() {
                    DataSlice::Range(content.parse()?)
                } else {
                    DataSlice::Index(content.parse::<syn::Expr>()?)
                });
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

impl Data {
    pub fn into_idl_arg(&self) -> syn::Result<IdlArg> {
        Ok(IdlArg {
            name: self.name.to_string(),
            r#type: rust_to_idl(&self.ty)?,
        })
    }
}

pub struct State {
    pub name: Ident,
    pub fields: Vec<Fields>,
}

pub struct Fields {
    pub name: Ident,
    pub ty: Type,
}

///Helper:

pub fn account_discriminator(struct_name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let preimage = format!("account:{struct_name}");
    Sha256::digest(preimage.as_bytes())[..8].try_into().unwrap()
}

pub fn derive_instruction_name(ident: &Ident) -> String {
    ident.to_string()
}

/*
pub fn rust_to_idl(ty: &Type) -> syn::Result<String> {
    match ty {
        Type::Path(p) => {
            let ident = p.path.segments.last().ok_or_else(|| syn::Error::new_spanned(p, "empty type path"))?
                .ident.to_string();

            Ok(match ident.as_str() {
                "Pubkey" => "pubkey".to_string(),
                other => other.to_string(),
            })
        }
        other => Err(syn::Error::new_spanned(
            other,
            "unsupported data type in #[p_instruction(...)], use a primitive or `Pubkey`",
        ))
    }
}
*/

pub fn rust_to_idl(ty: &Type) -> syn::Result<String> {
    match ty {
        Type::Path(p) => {
            let last_seg = p
                .path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(p, "empty type path"))?;
            let ident = last_seg.ident.to_string();

            match ident.as_str() {
                "Vec" => {
                    let inner = extract_arg(p, "Vec")?;
                    let inner_type = rust_to_idl(inner)?;
                    return Ok(format!("vec<{inner_type}>"));
                }
                "Option" => {
                    let inner = extract_arg(p, "Option")?;
                    let inner_type = rust_to_idl(inner)?;
                    return Ok(format!("option<{inner_type}>"));
                }
                _ => {}
            }

            Ok(match ident.as_str() {
                "Pubkey" | "Address" => "pubkey".to_string(),
                other => other.to_string(),
            })
        }

        Type::Array(arr) => {
            let elem_type = rust_to_idl(&arr.elem)?;
            let len = match &arr.len {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) => n.base10_parse::<usize>()?,
                _ => {
                    return Err(syn::Error::new_spanned(
                        &arr.len,
                        "array length must be a literal integer",
                    ));
                }
            };

            if elem_type == "u8" && len == 32 {
                Ok("pubkey".to_string())
            } else if elem_type == "u8" {
                Ok("bytes".to_string())
            } else {
                Ok(format!("[{}; {}]", elem_type, len))
            }
        }

        other => Err(syn::Error::new_spanned(
            other,
            "unsupported data type in #[p_instruction(...)] or #[p_state] use a primitive, `Pubkey`, `Address`, `Vec<T>`, or `Option<T>`",
        )),
    }
}

fn extract_arg<'a>(typ: &'a syn::TypePath, wrapper: &str) -> syn::Result<&'a Type> {
    let last = typ.path.segments.last().unwrap();

    if let syn::PathArguments::AngleBracketed(ref args) = last.arguments {
        let mut types = args.args.iter().filter_map(|a| {
            if let syn::GenericArgument::Type(t) = a {
                Some(t)
            } else {
                None
            }
        });

        if let Some(inner) = types.next() {
            return Ok(inner);
        }
    }
    Err(syn::Error::new_spanned(
        typ,
        format!("`{wrapper}` requires exactly one type argument"),
    ))
}

/*
fn seed_expr_to_idl(
    expr: &Expr,
    accounts_name: &[String],
    arg_name: &[String]
) -> syn::Result<IdlPdaSeed> {
    match strip(expr) {
        Expr::Lit(lit) => match &lit.lit {
            //             Lit::Str(s) => Ok(IdlPdaSeed::Const { value: s.value().into_bytes() }),
            Lit::Str(s) => Ok(IdlPdaSeed::Const { value: s.value().to_le_bytes }),
            Lit::ByteStr(b) => Ok(IdlPdaSeed::Const { value: b.value() }),
            other => Err(syn::Error::new_spanned(
                other, "pda seed literal must be a string or byte string",
            )),
        },


        Expr::Path(p) => {
            let ident = p.path.get_ident().ok_or_else(|| {
                syn::Error::new_spanned(p, "pda seed path must be a single identifier")
            })?;
            let name = ident.to_string();

            if accounts_name.iter().any(|a| a == &name) {
                Ok(IdlPdaSeed::Account { path: name, account: None})
            } else if arg_name.iter().any(|a| a == &name) {
                Ok(IdlPdaSeed::Arg { path: name })
            } else {
                Err(syn::Error::new_spanned(
                    p,
                    format!("seed `{name}` doesn't match any declared account or data field"),
                ))
            }
        }
        other => Err(syn::Error::new_spanned(
            other, "pda seed must be a string/byte-string literal or an identifier",
        )),
    }
}
*/

fn seed_expr_to_idl(
    expr: &Expr,
    account_names: &[String],
    arg_names: &[String],
) -> syn::Result<IdlPdaSeed> {
    Ok(match classify_seed(expr, account_names, arg_names)? {
        SeedClass::Bytes(value) => IdlPdaSeed::Const { value },
        SeedClass::Account(ident) => IdlPdaSeed::Account {
            path: ident.to_string(),
            account: None,
        },
        SeedClass::Arg(ident) => IdlPdaSeed::Arg {
            path: ident.to_string(),
        },
    })
}

fn is_pubkey_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.path.is_ident("Pubkey") || p.path.is_ident("Address"))
}

pub fn seed_class_to_tokens(class: &SeedClass, data_fields: &[Data]) -> syn::Result<TokenStream2> {
    Ok(match class {
        SeedClass::Bytes(bytes) => {
            let lit = syn::LitByteStr::new(bytes, proc_macro2::Span::call_site());

            quote! { #lit }
        }
        SeedClass::Account(ident) => quote! { #ident.address().as_ref() }, // #ident.key()

        SeedClass::Arg(ident) => {
            let field = data_fields
                .iter()
                .find(|f| &f.name == ident)
                .ok_or_else(|| {
                    syn::Error::new_spanned(ident, "internal: arg seed not found among data fields")
                })?;

            if is_pubkey_type(&field.ty) {
                quote! { #ident.as_ref() }
            } else {
                quote! { &#ident.to_le_bytes() }
            }
        }
    })
}

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
             (e.g. `accounts: &[AccountView]`), account bindings are located \
             by this exact name, regardless of position or type",
            sig.ident
        ),
    ))
}

/// Accounts.Parse the list of binded accounts and scan their results to ensure they don't...
/// already appear in the list of instruction.accounts the add them to ths list of instruction.accounts

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
            address: None,
            relations: Vec::new(),
        })
        .collect();

    instruction.accounts.extend(other_accounts);
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
