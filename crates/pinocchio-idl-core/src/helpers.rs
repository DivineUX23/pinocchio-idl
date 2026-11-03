use crate::{FieldType, IdlArg, IdlPdaSeed};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, FnArg, Ident, Lit, Pat, Signature, Stmt, Token, Type,
    parse::{Parse, ParseStream},
};

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
    pub slice: Option<DataSlice>,
}

impl Parse for Data {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        input.parse::<Token![:]>()?;
        let ty = input.parse()?;

        let mut slice = None;

        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let ident: Ident = input.parse()?;

            if ident.to_string().as_str() == "data" {
                let content;
                syn::bracketed!( content in input);

                slice = Some(if content.fork().parse::<syn::ExprRange>().is_ok() {
                    DataSlice::Range(content.parse()?)
                } else {
                    DataSlice::Index(content.parse::<syn::Expr>()?)
                });
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
        self.into_idl_arg_with_context(false)
    }

    pub fn into_idl_arg_with_context(&self, strict_pubkeys: bool) -> syn::Result<IdlArg> {
        Ok(IdlArg {
            name: self.name.to_string(),
            r#type: rust_to_idl_with_context(
                &self.ty,
                Some(&self.name.to_string()),
                strict_pubkeys,
            )?,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscriminatorSpec {
    Default,
    None,
    Bytes(Vec<u8>),
}

impl DiscriminatorSpec {
    pub fn to_account_bytes(&self, struct_name: &str) -> Vec<u8> {
        match self {
            DiscriminatorSpec::Default => account_discriminator(struct_name).to_vec(),
            DiscriminatorSpec::None => Vec::new(),
            DiscriminatorSpec::Bytes(b) => b.clone(),
        }
    }

    pub fn to_event_bytes(&self, struct_name: &str) -> Vec<u8> {
        match self {
            DiscriminatorSpec::Default => event_discriminator(struct_name).to_vec(),
            DiscriminatorSpec::None => Vec::new(),
            DiscriminatorSpec::Bytes(b) => b.clone(),
        }
    }
}

pub fn parse_discriminator_expr(expr: &syn::Expr) -> syn::Result<DiscriminatorSpec> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(n),
            ..
        }) => {
            let val = n.base10_parse::<u64>()?;
            if val <= 255 {
                Ok(DiscriminatorSpec::Bytes(vec![val as u8]))
            } else {
                let bytes = val.to_le_bytes();
                let len = bytes.iter().rposition(|&b| b != 0).map_or(1, |pos| pos + 1);
                Ok(DiscriminatorSpec::Bytes(bytes[..len].to_vec()))
            }
        }
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::ByteStr(bs),
            ..
        }) => Ok(DiscriminatorSpec::Bytes(bs.value())),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Ok(DiscriminatorSpec::Bytes(s.value().into_bytes())),
        syn::Expr::Array(syn::ExprArray { elems, .. }) => {
            if elems.is_empty() {
                return Ok(DiscriminatorSpec::None);
            }
            let mut bytes = Vec::new();
            for elem in elems {
                match elem {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(lit),
                        ..
                    }) => {
                        bytes.push(lit.base10_parse::<u8>()?);
                    }
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Byte(b),
                        ..
                    }) => {
                        bytes.push(b.value());
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "discriminator array elements must be integer or byte literals",
                        ));
                    }
                }
            }
            Ok(DiscriminatorSpec::Bytes(bytes))
        }
        syn::Expr::Path(p) if p.path.is_ident("none") => Ok(DiscriminatorSpec::None),
        other => Err(syn::Error::new_spanned(
            other,
            "unsupported discriminator expression; expected integer (e.g. 1), byte array (e.g. [1, 2, 3, 4]), string/bytes (e.g. b\"PORT\"), or `none`",
        )),
    }
}

pub fn extract_discriminator_spec(attrs: &[syn::Attribute], attr_name: &str) -> DiscriminatorSpec {
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let mut spec = DiscriminatorSpec::Default;
                let _ = meta_list.parse_nested_meta(|nested| {
                    if nested.path.is_ident("discriminator") {
                        if let Ok(value) = nested.value() {
                            if let Ok(expr) = value.parse::<syn::Expr>() {
                                if let Ok(parsed) = parse_discriminator_expr(&expr) {
                                    spec = parsed;
                                }
                            }
                        }
                    }
                    Ok(())
                });
                return spec;
            }
        }
    }
    DiscriminatorSpec::Default
}

pub fn extract_strict_pubkeys(attrs: &[syn::Attribute], attr_name: &str) -> bool {
    for attr in attrs {
        if attr.path().is_ident(attr_name) {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let mut strict = false;
                let _ = meta_list.parse_nested_meta(|nested| {
                    if nested.path.is_ident("strict_pubkeys")
                        || nested.path.is_ident("preserve_arrays")
                    {
                        strict = true;
                    }
                    Ok(())
                });
                if strict {
                    return true;
                }
            }
        }
    }
    false
}

pub fn is_hash_field_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "hash"
        || lower.ends_with("_hash")
        || lower.starts_with("hash_")
        || lower == "digest"
        || lower.ends_with("_digest")
        || lower.starts_with("digest_")
        || lower == "signature"
        || lower.ends_with("_signature")
        || lower.starts_with("sig_")
        || lower == "merkle_root"
        || lower.ends_with("_root")
        || lower == "salt"
        || lower == "secret"
        || lower.ends_with("_bytes")
        || lower.starts_with("bytes_")
        || lower == "bytes"
        || lower == "payload"
}

pub fn account_discriminator(struct_name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let preimage = format!("account:{struct_name}");
    Sha256::digest(preimage.as_bytes())[..8].try_into().unwrap()
}

pub fn event_discriminator(struct_name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let preimage = format!("event:{struct_name}");
    Sha256::digest(preimage.as_bytes())[..8].try_into().unwrap()
}

pub fn extract_docs(attrs: &[syn::Attribute]) -> Option<Vec<String>> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            let nv = attr.meta.require_name_value().ok()?;
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                let trimmed = s.value().trim().to_string();
                if !trimmed.is_empty() {
                    return Some(trimmed);
                }
            }
            None
        })
        .collect();

    if docs.is_empty() { None } else { Some(docs) }
}

pub fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "u256"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "f32"
            | "f64"
            | "string"
            | "String"
            | "str"
            | "Pubkey"
            | "Address"
            | "pubkey"
            | "bytes"
    )
}

pub fn derive_instruction_name(ident: &Ident) -> String {
    ident.to_string()
}

pub fn rust_to_idl(ty: &Type) -> syn::Result<FieldType> {
    rust_to_idl_with_context(ty, None, false)
}

pub fn rust_to_idl_with_context(
    ty: &Type,
    field_name: Option<&str>,
    strict_pubkeys: bool,
) -> syn::Result<FieldType> {
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
                    let inner_type = rust_to_idl_with_context(inner, field_name, strict_pubkeys)?;
                    return Ok(FieldType::Vec(Box::new(inner_type)));
                }
                "Option" => {
                    let inner = extract_arg(p, "Option")?;
                    let inner_type = rust_to_idl_with_context(inner, field_name, strict_pubkeys)?;
                    return Ok(FieldType::Option(Box::new(inner_type)));
                }
                _ => {}
            }

            Ok(match ident.as_str() {
                "Pubkey" | "Address" => FieldType::Simple("pubkey".to_string()),
                "String" | "str" => FieldType::Simple("string".to_string()),
                s if is_primitive_type(s) => FieldType::Simple(s.to_string()),
                custom => FieldType::Defined(custom.to_string()),
            })
        }

        Type::Array(arr) => {
            let elem_type = rust_to_idl_with_context(&arr.elem, field_name, strict_pubkeys)?;
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

            if elem_type == FieldType::Simple("u8".to_string()) && len == 32 {
                let is_hash = field_name.is_some_and(is_hash_field_name);
                if strict_pubkeys || is_hash {
                    Ok(FieldType::Array(Box::new(elem_type), len))
                } else {
                    Ok(FieldType::Simple("pubkey".to_string()))
                }
            } else {
                Ok(FieldType::Array(Box::new(elem_type), len))
            }
        }

        Type::Reference(r) => rust_to_idl_with_context(&r.elem, field_name, strict_pubkeys),

        Type::Slice(s) => {
            let inner = rust_to_idl_with_context(&s.elem, field_name, strict_pubkeys)?;
            if inner == FieldType::Simple("u8".to_string()) {
                Ok(FieldType::Simple("bytes".to_string()))
            } else {
                Ok(FieldType::Vec(Box::new(inner)))
            }
        }

        other => Err(syn::Error::new_spanned(
            other,
            "unsupported data type in #[p_instruction(...)], #[p_state] or #[p_constant], use a primitive, `Pubkey`, `Address`, `Vec<T>`, `Option<T>`, &T, [T], or [T; N]",
        )),
    }
}

pub fn rust_to_idl_with_field(field: &syn::Field, strict_pubkeys: bool) -> syn::Result<FieldType> {
    let has_bytes_attr = field.attrs.iter().any(|attr| {
        attr.path().is_ident("p_bytes")
            || attr.path().is_ident("p_hash")
            || attr.path().is_ident("p_array")
    });
    let has_pubkey_attr = field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("p_pubkey"));

    let field_name = field.ident.as_ref().map(|i| i.to_string());

    if has_pubkey_attr {
        Ok(FieldType::Simple("pubkey".to_string()))
    } else if has_bytes_attr {
        rust_to_idl_with_context(&field.ty, field_name.as_deref(), true)
    } else {
        rust_to_idl_with_context(&field.ty, field_name.as_deref(), strict_pubkeys)
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

pub fn seed_expr_to_idl(
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

pub fn is_pubkey_type(ty: &Type) -> bool {
    //matches!(ty, Type::Path(p) if p.path.is_ident("Pubkey") || p.path.is_ident("Address"))

    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Pubkey" || s.ident == "Address"))
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
/*
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
            pda: None,
            ata: None,
            struct_state: None,
            address: None,
            relations: Vec::new(),
        })
        .collect();

    instruction.accounts.extend(other_accounts);
}
*/

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

fn is_get_account(mut expr: &Expr, name: &str) -> bool {
    let mut has_get = false;
    loop {
        expr = strip(expr);
        match expr {
            Expr::Try(t) => expr = &t.expr,
            Expr::MethodCall(m) => {
                //let method_name = m.method.to_string();
                if m.method == "get" || m.method == "get_mut" || m.method == "next" {
                    has_get = true;
                }
                expr = &m.receiver;
            }
            Expr::Path(p) => return has_get && p.path.is_ident(name),
            _ => return false,
        }
    }
}

pub fn account_binding(stmts: &[Stmt], accounts_param: &str) -> Vec<Ident> {
    let mut binding = Vec::new();

    for stmt in stmts {
        if let Stmt::Local(local) = stmt {
            let mut pat = &local.pat;
            if let Pat::Type(pat_type) = pat {
                pat = &pat_type.pat;
            }

            if let Pat::Slice(slice) = pat {
                if let Some(init) = &local.init {
                    if is_path_ident(&init.expr, accounts_param) {
                        //let binding = slice.elems.iter().filter(|p| !matches!(p, Pat::Rest(_))).collect();
                        for p in slice.elems.iter() {
                            if let Pat::Ident(pat_ident) = p {
                                if pat_ident.subpat.is_none() {
                                    binding.push(pat_ident.ident.clone());
                                }
                            }
                        }
                    }
                }
            }

            if let Some(init) = &local.init {
                if is_indexed_account(&init.expr, accounts_param)
                    || is_get_account(&init.expr, accounts_param)
                {
                    if let Pat::Ident(pat_ident) = pat {
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

pub fn count_account_binding(stmts: &[Stmt], accounts_param: &str) -> usize {
    let mut index = 0;

    for (i, stmt) in stmts.iter().enumerate() {
        if let Stmt::Local(local) = stmt {
            if let Some(init) = &local.init {
                if let Pat::Slice(_slice) = &local.pat {
                    if is_path_ident(&init.expr, accounts_param) {
                        //return slice.elems.iter().filter(|p| !matches!(p, Pat::Rest(_))).count();
                        return i + 1;
                    }
                }

                if is_indexed_account(&init.expr, accounts_param)
                    || is_get_account(&init.expr, accounts_param)
                {
                    index = i + 1;
                }
            }
        } else {
            break;
        }
    }
    index
}

pub fn bs58_decode(s: &str) -> Result<Vec<u8>, String> {
    bs58::decode(s)
        .into_vec()
        .map_err(|e| format!("invalid Base58: {}", e))
}

/*

pub fn bs58_decode(s: &str) -> Result<Vec<u8>, String> {
    // Simple Base58 decoder (alphabet: Bitcoin/Solana)
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut digits: Vec<u8> = s
        .bytes()
        .map(|b| {
            ALPHABET
                .iter()
                .position(|&a| a == b)
                .ok_or_else(|| format!("invalid Base58 character: {b:?}"))
                .map(|p| p as u8)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Convert base-58 digits to big-endian bytes
    let mut out: Vec<u8> = Vec::new();
    while !digits.is_empty() {
        let mut rem = 0u32;
        let mut new_digits = Vec::new();
        for &d in &digits {
            let cur = rem * 58 + d as u32;
            if !new_digits.is_empty() || cur / 256 > 0 {
                new_digits.push((cur / 256) as u8);
            }
            rem = cur % 256;
        }
        out.push(rem as u8);
        digits = new_digits;
    }

    // Count leading '1' characters (= 0x00 prefix bytes)
    let leading_zeros = s.bytes().take_while(|&b| b == b'1').count();
    let mut result = Vec::new();
    result.extend(std::iter::repeat(0u8).take(leading_zeros));
    result.extend(out.iter().rev());
    Ok(result)
}

*/
