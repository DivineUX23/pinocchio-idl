use quote::ToTokens;
use std::{fs, path::Path};
use syn::{Fields, ItemConst, ItemEnum, ItemStruct, Lit};


use pinocchio_idl_core::{
    Idl, IdlAccountDef, IdlConstant, IdlError, IdlField, IdlType, IdlTypeDefinition, Instruction,
    Metadata, account_discriminator, derive_instruction_name, find_accounts_param, rust_to_idl,
};

use crate::discover::discover;

fn format_syn_error(err: syn::Error, file: &Path) -> anyhow::Error {
    let mut msgs = Vec::new();
    for e in err.into_iter() {
        let span = e.span().start();
        msgs.push(format!("{} at {}:{}:{}", e, file.display(), span.line, span.column));
    }
    anyhow::anyhow!("{}", msgs.join("\n"))
}

fn state_to_idl(item: &ItemStruct) -> syn::Result<(IdlAccountDef, IdlTypeDefinition)> {
    let name = item.ident.to_string();

    let fields = match &item.fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| {
                let field_name = f
                    .ident
                    .as_ref()
                    .ok_or_else(|| syn::Error::new_spanned(f, "state fields must be named"))?
                    .to_string();

                Ok(IdlField {
                    name: field_name,
                    r#type: rust_to_idl(&f.ty)?,
                })
            })
            .collect::<syn::Result<Vec<_>>>()?,
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "#[p_state] requires named fields",
            ));
        }
    };

    Ok((
        IdlAccountDef {
            name: name.clone(),
            discriminator: account_discriminator(&name).to_vec(),
        },
        IdlTypeDefinition {
            name,
            r#type: IdlType {
                kind: "struct".to_string(),
                fields,
            },
        },
    ))
}

fn errors_from_enum(item: &ItemEnum) -> syn::Result<Vec<IdlError>> {
    let mut errors = Vec::new();

    for (default_code, variant) in item.variants.iter().enumerate() {
        let name = variant.ident.to_string();

        let msg: Option<String> = {
            let parts: Vec<String> = variant
                .attrs
                .iter()
                .filter_map(|attr| {
                    if !attr.path().is_ident("doc") {
                        return None;
                    }
                    if let syn::Meta::NameValue(nv) = &attr.meta {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(s), ..
                        }) = &nv.value
                        {
                            let trimmed = s.value().trim().to_string();
                            if !trimmed.is_empty() {
                                Some(trimmed)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" "))
            }
        };

        // for overide default
        let code: u32 = variant
            .attrs
            .iter()
            .find_map(|attr| {
                if !attr.path().is_ident("p_code") {
                    return None;
                }
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: Lit::Int(n), ..
                    }) = &nv.value
                    {
                        n.base10_parse::<u32>().ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or(default_code as u32);

        errors.push(IdlError {
            code,
            name: name.clone(),
            msg: msg.unwrap_or(name),
        });
    }

    Ok(errors)
}

fn constant_from_item(item: &ItemConst) -> syn::Result<IdlConstant> {
    let name = item.ident.to_string();
    let ty = rust_to_idl(&item.ty)?;

    let value = item.expr.to_token_stream().to_string();

    Ok(IdlConstant {
        name,
        r#type: ty,
        value,
    })
}

pub fn build_idl(src_dir: &Path, metadata: Metadata) -> anyhow::Result<Idl> {
    let discovery = discover(src_dir).map_err(|e| anyhow::anyhow!("Discovery error: {}", e))?;

    if discovery.instructions.is_empty() && discovery.states.is_empty() {
        anyhow::bail!("No #[p_instruction] or #[p_state] annotations found in {}", src_dir.display());
    }

    let instructions = discovery
        .instructions
        .iter()
        .enumerate()
        .map(|(index, discovered)| {
            let mut instruction: Instruction = syn::parse2(discovered.attr_tokens.clone())
                .map_err(|e| format_syn_error(e, &discovered.file))?;

            let accounts_ident = find_accounts_param(&discovered.func.sig)
                .map_err(|e| format_syn_error(e, &discovered.file))?;
            instruction.add_accounts(&discovered.func.block.stmts, &accounts_ident.to_string());

            let name = derive_instruction_name(&discovered.func.sig.ident);
            instruction.into_idl(name, index as u8)
                .map_err(|e| format_syn_error(e, &discovered.file))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let (accounts, types): (Vec<_>, Vec<_>) = discovery
        .states
        .iter()
        .map(|(s, file)| state_to_idl(s).map_err(|e| format_syn_error(e, file)))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .unzip();

    let errors: Vec<IdlError> = discovery
        .errors
        .iter()
        .map(|(e, file)| errors_from_enum(e).map_err(|err| format_syn_error(err, file)))
        .collect::<anyhow::Result<Vec<Vec<_>>>>()?
        .into_iter()
        .flatten()
        .collect();

    let constants: Vec<IdlConstant> = discovery
        .constants
        .iter()
        .map(|(c, file)| constant_from_item(c).map_err(|e| format_syn_error(e, file)))
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(Idl {
        address: discovery.program_id.unwrap_or_default(),
        metadata,
        instructions,
        accounts,
        errors,
        types,
        constants,
    })
}

pub fn write_idl(idl: &Idl, out_path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(idl).expect("Idl serialization is infallible");
    fs::write(out_path, json)
}
