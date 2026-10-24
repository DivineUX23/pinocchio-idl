use quote::ToTokens;
use std::{fs, path::Path};
use syn::{Fields, Item, ItemConst, ItemEnum, ItemStruct, Lit};

use anyhow::Context;
use pinocchio_idl_core::{
    Idl, IdlAccountDef, IdlConstant, IdlEnumVariant, IdlError, IdlField, IdlType,
    IdlTypeDefinition, Instruction, Metadata, account_discriminator, derive_instruction_name,
    extract_docs, find_accounts_param, rust_to_idl,
};

use crate::discover::discover;

fn format_syn_error(err: syn::Error, file: &Path) -> anyhow::Error {
    let msgs: Vec<String> = err
        .into_iter()
        .map(|e| {
            let span = e.span().start();
            format!("{} at {}:{}:{}", e, file.display(), span.line, span.column)
        })
        .collect();
    anyhow::anyhow!("{}", msgs.join("\n"))
}

fn state_to_idl(item: &ItemStruct) -> syn::Result<(IdlAccountDef, IdlTypeDefinition)> {
    let name = item.ident.to_string();
    let docs = extract_docs(&item.attrs);

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
                    docs: extract_docs(&f.attrs),
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
            docs: docs.clone(),
            discriminator: account_discriminator(&name).to_vec(),
        },
        IdlTypeDefinition {
            name,
            docs,
            r#type: IdlType {
                kind: "struct".to_string(),
                fields,
                variants: Vec::new(),
            },
        },
    ))
}

fn event_to_idl(
    item: &ItemStruct,
) -> syn::Result<(pinocchio_idl_core::IdlEvent, IdlTypeDefinition)> {
    let name = item.ident.to_string();
    let docs = extract_docs(&item.attrs);

    let fields = match &item.fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| {
                let field_name = f
                    .ident
                    .as_ref()
                    .ok_or_else(|| syn::Error::new_spanned(f, "event fields must be named"))?
                    .to_string();

                Ok(IdlField {
                    name: field_name,
                    docs: extract_docs(&f.attrs),
                    r#type: rust_to_idl(&f.ty)?,
                })
            })
            .collect::<syn::Result<Vec<_>>>()?,
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "#[p_event] requires named fields",
            ));
        }
    };

    Ok((
        pinocchio_idl_core::IdlEvent {
            name: name.clone(),
            discriminator: pinocchio_idl_core::event_discriminator(&name).to_vec(),
        },
        IdlTypeDefinition {
            name,
            docs,
            r#type: IdlType {
                kind: "struct".to_string(),
                fields,
                variants: Vec::new(),
            },
        },
    ))
}

pub fn type_item_to_idl(item: &Item) -> syn::Result<IdlTypeDefinition> {
    match item {
        Item::Struct(s) => {
            let name = s.ident.to_string();
            let docs = extract_docs(&s.attrs);

            let fields = match &s.fields {
                Fields::Named(named) => named
                    .named
                    .iter()
                    .map(|f| {
                        let field_name = f
                            .ident
                            .as_ref()
                            .ok_or_else(|| syn::Error::new_spanned(f, "type fields must be named"))?
                            .to_string();

                        Ok(IdlField {
                            name: field_name,
                            docs: extract_docs(&f.attrs),
                            r#type: rust_to_idl(&f.ty)?,
                        })
                    })
                    .collect::<syn::Result<Vec<_>>>()?,
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "#[p_type] structs require named fields",
                    ));
                }
            };

            Ok(IdlTypeDefinition {
                name,
                docs,
                r#type: IdlType {
                    kind: "struct".to_string(),
                    fields,
                    variants: Vec::new(),
                },
            })
        }
        Item::Enum(e) => {
            let name = e.ident.to_string();
            let docs = extract_docs(&e.attrs);

            let mut variants = Vec::new();
            for variant in &e.variants {
                let v_name = variant.ident.to_string();
                let v_docs = extract_docs(&variant.attrs);

                let v_fields = match &variant.fields {
                    Fields::Unit => None,
                    Fields::Named(named) => {
                        let fields = named
                            .named
                            .iter()
                            .map(|f| {
                                let f_name =
                                    f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                                Ok(IdlField {
                                    name: f_name,
                                    docs: extract_docs(&f.attrs),
                                    r#type: rust_to_idl(&f.ty)?,
                                })
                            })
                            .collect::<syn::Result<Vec<_>>>()?;
                        Some(fields)
                    }
                    Fields::Unnamed(unnamed) => {
                        let fields = unnamed
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, f)| {
                                Ok(IdlField {
                                    name: format!("field{i}"),
                                    docs: extract_docs(&f.attrs),
                                    r#type: rust_to_idl(&f.ty)?,
                                })
                            })
                            .collect::<syn::Result<Vec<_>>>()?;
                        Some(fields)
                    }
                };

                variants.push(IdlEnumVariant {
                    name: v_name,
                    docs: v_docs,
                    fields: v_fields,
                });
            }

            Ok(IdlTypeDefinition {
                name,
                docs,
                r#type: IdlType {
                    kind: "enum".to_string(),
                    fields: Vec::new(),
                    variants,
                },
            })
        }
        other => Err(syn::Error::new_spanned(
            other,
            "#[p_type] can only be applied to a struct or enum",
        )),
    }
}

fn errors_from_enum(item: &ItemEnum) -> syn::Result<Vec<IdlError>> {
    let mut errors = Vec::new();

    for (index, variant) in item.variants.iter().enumerate() {
        let name = variant.ident.to_string();

        let parts: Vec<String> = variant
            .attrs
            .iter()
            .filter_map(|attr| {
                if !attr.path().is_ident("doc") {
                    return None;
                }
                let nv = attr.meta.require_name_value().ok()?;

                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
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

        let msg = if parts.is_empty() {
            name.clone()
        } else {
            parts.join(" ")
        };

        let code: u32 = variant
            .attrs
            .iter()
            .find_map(|attr| {
                if !attr.path().is_ident("p_code") {
                    return None;
                }
                let nv = attr.meta.require_name_value().ok()?;
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Int(n), ..
                }) = &nv.value
                {
                    n.base10_parse::<u32>().ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| 6000 + index as u32);

        errors.push(IdlError { code, name, msg });
    }
    Ok(errors)
}
/*

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
*/

fn constant_from_item(item: &ItemConst) -> syn::Result<IdlConstant> {
    let name = item.ident.to_string();
    let docs = extract_docs(&item.attrs);
    let ty = rust_to_idl(&item.ty)?;

    let value = item.expr.to_token_stream().to_string();

    Ok(IdlConstant {
        name,
        docs,
        r#type: ty,
        value,
    })
}

pub fn build_idl(src_dir: &Path, metadata: Metadata) -> anyhow::Result<Idl> {
    let discovery = discover(src_dir).map_err(|e| anyhow::anyhow!("Discovery error: {}", e))?;

    if discovery.instructions.is_empty()
        && discovery.states.is_empty()
        && discovery.types.is_empty()
    {
        anyhow::bail!(
            "No #[p_instruction], #[p_state], or #[p_type] annotations found in {}",
            src_dir.display()
        );
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
            let docs = extract_docs(&discovered.func.attrs);
            instruction
                .into_idl_with_docs(name, index as u8, docs)
                .map_err(|e| format_syn_error(e, &discovered.file))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let (accounts, mut types): (Vec<_>, Vec<_>) = discovery
        .states
        .iter()
        .map(|(s, file)| state_to_idl(s).map_err(|e| format_syn_error(e, file)))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .unzip();

    let (events, event_types): (Vec<_>, Vec<_>) = discovery
        .events
        .iter()
        .map(|(e, file)| event_to_idl(e).map_err(|err| format_syn_error(err, file)))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .unzip();

    types.extend(event_types);

    let custom_types: Vec<IdlTypeDefinition> = discovery
        .types
        .iter()
        .map(|(item, file)| type_item_to_idl(item).map_err(|err| format_syn_error(err, file)))
        .collect::<anyhow::Result<Vec<_>>>()?;

    types.extend(custom_types);

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
        events,
    })
}

pub fn write_idl(idl: &Idl, out_path: &Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(idl).context("Failed to serialize IDL to JSON")?;

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    fs::write(out_path, &json)
        .with_context(|| format!("Failed to write IDL to {}", out_path.display()))?;

    Ok(())
}
