use pinocchio_idl_core::{Data, SeedClass, is_pubkey_type};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/*
pub fn strip(mut expr: &Expr) -> &Expr {
    loop {
        expr = match expr {
            Expr::Reference(r) => &r.expr,
            Expr::Paren(p) => &p.expr,
            Expr::Group(g) => &g.expr,
            _ => return expr,
        }
    }
}

pub fn is_path_ident(expr: &Expr, name: &str) -> bool {
    matches!(strip(expr), Expr::Path(p) if p.path.is_ident(name))
}

pub fn is_indexed_account(expr: &Expr, name: &str) -> bool {
    match strip(expr) {
        Expr::Index(idx) => is_path_ident(&idx.expr, name),
        _ => false,
    }
}

pub fn is_get_account(mut expr: &Expr, name: &str) -> bool {
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


 */
/*fn is_pubkey_type(ty: &Type) -> bool {
    //matches!(ty, Type::Path(p) if p.path.is_ident("Pubkey") || p.path.is_ident("Address"))

    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Pubkey" || s.ident == "Address"))
}*/

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

/*
pub fn field_byte_size(ty: &Type) -> syn::Result<usize> {
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
                    // 4-byte length prefix + heap data
                    return Ok(4);
                }
                "Option" => {
                    // 1-byte discriminant + inner type size
                    let inner = extract_single_type_arg(p).ok_or_else(|| {
                        syn::Error::new_spanned(p, "`Option` requires one type argument")
                    })?;
                    return Ok(1 + field_byte_size(inner)?);
                }
                _ => {}
            }

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
                        "#[p_state] doesn't know the size of `{other}`, \
                                use an integer, bool, Pubkey, Address, Vec<T>, Option<T>, or a fixed-size array. \
                                For enum/struct types, annotate with an explicit byte size or use a primitive."
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

pub fn extract_single_type_arg(p: &syn::TypePath) -> Option<&Type> {
    let last = p.path.segments.last()?;
    if let syn::PathArguments::AngleBracketed(ref args) = last.arguments {
        for arg in &args.args {
            if let syn::GenericArgument::Type(t) = arg {
                return Some(t);
            }
        }
    }
    None
}
*/
