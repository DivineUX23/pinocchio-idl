use serde_json::{Value, json};
use std::collections::HashSet;

use crate::{
    FieldType, Idl, IdlAccount, IdlArg, IdlField, IdlInstruction, IdlPdaSeed, IdlTypeDefinition,
};

pub fn to_codama_json(idl: &Idl) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&codama_root(idl))
}

fn codama_root(idl: &Idl) -> Value {
    let pdas = collect_program_pdas(idl);
    json!({
        "kind": "rootNode",
        "standard": "codama",
        "version": "1.6.0",
        "program": {
            "kind": "programNode",
            "name": camel_case(&idl.metadata.name),
            "publicKey": idl.address,
            "version": idl.metadata.version,
            "origin": "pinocchio-idl",
            "accounts": idl.accounts.iter()
                .filter_map(|acc| idl.types.iter().find(|t| t.name == acc.name).map(|ty| codama_account(acc, ty)))
                .collect::<Vec<_>>(),
            "instructions": idl.instructions.iter().map(codama_instruction).collect::<Vec<_>>(),

            "definedTypes": idl.types.iter()
                .filter(|t| !idl.accounts.iter().any(|a| a.name == t.name))
                .filter(|t| !idl.events.iter().any(|e| e.name == t.name))
                .map(codama_defined_type)
                .collect::<Vec<_>>(),

            "pdas": pdas,
            "errors": idl.errors.iter().map(|e| json!({
                "kind": "errorNode",
                "code": e.code,
                "name": camel_case(&e.name),
                "message": e.msg,
            })).collect::<Vec<_>>(),
            "constants": idl.constants.iter().map(|c| json!({
                "kind": "constantNode",
                "name": camel_case(&c.name),
                "type": codama_field_type(&c.r#type),
                "value": {
                    "kind": "numberValueNode",
                    "number": eval_const_number(&c.value),
                },
            })).collect::<Vec<_>>(),
        },
        "events": idl.events.iter().map(|e| {
            let ty = idl.types.iter().find(|t| t.name == e.name).unwrap();
            codama_event(e, ty)
        }).collect::<Vec<_>>(),
        "additionalPrograms": [],
    })
}

fn codama_event(event: &crate::IdlEvent, ty: &IdlTypeDefinition) -> Value {
    let discriminator_bytes = &event.discriminator;

    let fields: Vec<Value> = std::iter::once(codama_discriminator_field(discriminator_bytes))
        .chain(ty.r#type.fields.iter().map(codama_struct_field))
        .collect();

    json!({
        "kind": "eventNode",
        "name": camel_case(&event.name),
        "data": {
            "kind": "structTypeNode",
            "fields": fields,
        },
        "discriminators": [{
            "kind": "constantDiscriminatorNode",
            "offset": 0,
            "constant": {
                "kind": "constantValueNode",
                "type": {
                    "kind": "arrayTypeNode",
                    "item": { "kind": "numberTypeNode", "format": "u8" },
                    "count": { "kind": "fixedCountNode", "value": discriminator_bytes.len() }
                },
                "value": {
                    "kind": "arrayValueNode",
                    "items": discriminator_bytes.iter().map(|b| json!({
                        "kind": "numberValueNode",
                        "number": *b
                    })).collect::<Vec<_>>()
                }
            }
        }]
    })
}

fn codama_defined_type(ty: &IdlTypeDefinition) -> Value {
    let type_node = match ty.r#type.kind.as_str() {
        "enum" => {
            let variants = ty
                .r#type
                .variants
                .iter()
                .map(|v| {
                    let mut node = if let Some(fields) = &v.fields {
                        if !fields.is_empty() {
                            json!({
                                "kind": "enumStructVariantTypeNode",
                                "name": camel_case(&v.name),
                                "struct": {
                                    "kind": "structTypeNode",
                                    "fields": fields.iter().map(codama_struct_field).collect::<Vec<_>>(),
                                }
                            })
                        } else {
                            json!({
                                "kind": "enumEmptyVariantTypeNode",
                                "name": camel_case(&v.name),
                            })
                        }
                    } else {
                        json!({
                            "kind": "enumEmptyVariantTypeNode",
                            "name": camel_case(&v.name),
                        })
                    };
                    if let Some(docs) = &v.docs {
                        node["docs"] = json!(docs);
                    }
                    node
                })
                .collect::<Vec<_>>();

            json!({
                "kind": "enumTypeNode",
                "variants": variants,
                "size": {
                    "kind": "numberTypeNode",
                    "format": "u8",
                    "endian": "le"
                }
            })
        }
        _ => json!({
            "kind": "structTypeNode",
            "fields": ty.r#type.fields.iter().map(codama_struct_field).collect::<Vec<_>>(),
        }),
    };

    let mut node = json!({
        "kind": "definedTypeNode",
        "name": camel_case(&ty.name),
        "type": type_node,
    });

    if let Some(docs) = &ty.docs {
        node["docs"] = json!(docs);
    }

    node
}

fn codama_account(account: &crate::IdlAccountDef, ty: &IdlTypeDefinition) -> Value {
    let discriminator_bytes = &account.discriminator;

    let fields: Vec<Value> = std::iter::once(codama_discriminator_field(discriminator_bytes))
        .chain(ty.r#type.fields.iter().map(codama_struct_field))
        .collect();

    let mut node = json!({
        "kind": "accountNode",
        "name": camel_case(&account.name),
        "data": {
            "kind": "structTypeNode",
            "fields": fields,
        },
        "discriminators": [{
            "kind": "constantDiscriminatorNode",
            "offset": 0,
            "constant": {
                "kind": "constantValueNode",
                "type": fixed_bytes_type(discriminator_bytes.len()),
                "value": {
                    "kind": "bytesValueNode",
                    "data": hex_bytes(discriminator_bytes),
                    "encoding": "base16",
                },
            },
        }],
    });

    if let Some(docs) = &account.docs {
        node["docs"] = json!(docs);
    }

    node
}

fn codama_instruction(instruction: &IdlInstruction) -> Value {
    let discriminator_bytes = &instruction.discriminator;
    let args = instruction.args.as_deref().unwrap_or(&[]);
    let mut arguments = vec![codama_discriminator_argument(discriminator_bytes)];
    arguments.extend(args.iter().map(codama_instruction_argument));

    let mut node = json!({
        "kind": "instructionNode",
        "name": camel_case(&instruction.name),
        "optionalAccountStrategy": "omitted",
        "accounts": instruction.accounts.iter().map(|acc| codama_instruction_account(acc, args)).collect::<Vec<_>>(),
        "arguments": arguments,
        "discriminators": [{
            "kind": "fieldDiscriminatorNode",
            "name": "discriminator",
            "offset": 0,
        }],
    });

    if let Some(docs) = &instruction.docs {
        node["docs"] = json!(docs);
    }

    node
}

fn codama_instruction_account(account: &IdlAccount, _args: &[IdlArg]) -> Value {
    let mut value = json!({
        "kind": "instructionAccountNode",
        "name": camel_case(&account.name),
        "isWritable": account.writable,
        "isSigner": account.signer,
        "isOptional": account.optional,
    });

    if let Some(docs) = &account.docs {
        value["docs"] = json!(docs);
    }

    if let Some(addr) = &account.address {
        value["defaultValue"] = json!({
            "kind": "publicKeyValueNode",
            "publicKey": addr,
        });
    }

    // If this account has a PDA, link it to the program-level pdaNode so Codama
    // can generate `findXxxPda()` helpers and auto-resolve the address.
    if account.pda.is_some() {
        value["pda"] = json!({
            "kind": "pdaLinkNode",
            "name": camel_case(&account.name),
        });
    }

    value
}

fn codama_instruction_argument(arg: &IdlArg) -> Value {
    json!({
        "kind": "instructionArgumentNode",
        "name": camel_case(&arg.name),
        "type": codama_field_type(&arg.r#type),
    })
}

fn codama_discriminator_argument(discriminator: &[u8]) -> Value {
    let (disc_type, disc_value) = if discriminator.len() == 1 {
        (
            json!({ "kind": "numberTypeNode", "format": "u8", "endian": "le" }),
            json!({ "kind": "numberValueNode", "number": discriminator[0] }),
        )
    } else {
        (
            fixed_bytes_type(discriminator.len()),
            json!({
                "kind": "bytesValueNode",
                "data": hex_bytes(discriminator),
                "encoding": "base16",
            }),
        )
    };

    json!({
        "kind": "instructionArgumentNode",
        "name": "discriminator",
        "defaultValueStrategy": "omitted",
        "type": disc_type,
        "defaultValue": disc_value,
    })
}

fn codama_discriminator_field(discriminator: &[u8]) -> Value {
    let (disc_type, disc_value) = if discriminator.len() == 1 {
        (
            json!({ "kind": "numberTypeNode", "format": "u8", "endian": "le" }),
            json!({ "kind": "numberValueNode", "number": discriminator[0] }),
        )
    } else {
        (
            fixed_bytes_type(discriminator.len()),
            json!({
                "kind": "bytesValueNode",
                "data": hex_bytes(discriminator),
                "encoding": "base16",
            }),
        )
    };

    json!({
        "kind": "structFieldTypeNode",
        "name": "discriminator",
        "defaultValueStrategy": "omitted",
        "type": disc_type,
        "defaultValue": disc_value,
    })
}

fn codama_struct_field(field: &IdlField) -> Value {
    let mut node = json!({
        "kind": "structFieldTypeNode",
        "name": camel_case(&field.name),
        "type": codama_field_type(&field.r#type),
    });

    if let Some(docs) = &field.docs {
        node["docs"] = json!(docs);
    }

    node
}

fn codama_field_type(ty: &FieldType) -> Value {
    match ty {
        FieldType::Simple(name) => match name.as_str() {
            "bool" => json!({ "kind": "booleanTypeNode", "size": 1 }),
            "u8" => number_type("u8"),
            "u16" => number_type("u16"),
            "u32" => number_type("u32"),
            "u64" => number_type("u64"),
            "u128" => number_type("u128"),
            "i8" => number_type("i8"),
            "i16" => number_type("i16"),
            "i32" => number_type("i32"),
            "i64" => number_type("i64"),
            "i128" => number_type("i128"),
            "pubkey" | "publicKey" => json!({ "kind": "publicKeyTypeNode" }),
            "bytes" => json!({ "kind": "bytesTypeNode" }),
            "string" => json!({ "kind": "stringTypeNode", "encoding": "utf8" }),
            other => json!({ "kind": "definedTypeLinkNode", "name": camel_case(other) }),
        },
        FieldType::Array(inner, len) => {
            // [u8; N] → fixedSizeTypeNode wrapping bytesTypeNode
            if matches!(inner.as_ref(), FieldType::Simple(s) if s == "u8") {
                fixed_bytes_type(*len)
            } else {
                json!({
                    "kind": "arrayTypeNode",
                    "item": codama_field_type(inner),
                    "count": {
                        "kind": "fixedCountNode",
                        "value": len,
                    },
                })
            }
        }
        FieldType::Vec(inner) => json!({
            "kind": "arrayTypeNode",
            "item": codama_field_type(inner),
            "count": { "kind": "prefixedCountNode", "prefix": number_type("u32") },
        }),
        FieldType::Option(inner) => json!({
            "kind": "optionTypeNode",
            "item": codama_field_type(inner),
            "prefix": number_type("u8"),
        }),
        FieldType::Defined(name) => json!({
            "kind": "definedTypeLinkNode",
            "name": camel_case(name),
        }),
    }
}

fn number_type(format: &str) -> Value {
    json!({
        "kind": "numberTypeNode",
        "format": format,
        "endian": "le",
    })
}

fn fixed_bytes_type(size: usize) -> Value {
    json!({
        "kind": "fixedSizeTypeNode",
        "size": size,
        "type": { "kind": "bytesTypeNode" },
    })
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn eval_expr(s: &str) -> Result<i64, ()> {
    let mut sum: i64 = 0;
    for add_part in s.split('+') {
        let mut prod: i64 = 1;
        for mul_part in add_part.split('*') {
            let part = mul_part
                .trim()
                .trim_end_matches("u64")
                .trim_end_matches("i64")
                .trim_end_matches("u32")
                .trim_end_matches("i32")
                .trim_end_matches("u16")
                .trim_end_matches("i16")
                .trim_end_matches("u8")
                .trim_end_matches("i8")
                .trim_end_matches("usize")
                .trim();
            if part.is_empty() {
                return Err(());
            }
            let n: i64 = part.parse().map_err(|_| ())?;
            prod = prod.checked_mul(n).ok_or(())?;
        }
        sum = sum.checked_add(prod).ok_or(())?;
    }
    Ok(sum)
}

fn eval_const_number(val_str: &str) -> Value {
    let trimmed = val_str.trim().trim_matches('"');
    if let Ok(num) = eval_expr(trimmed) {
        return json!(num);
    }
    if let Ok(num) = trimmed.parse::<i64>() {
        return json!(num);
    }
    json!(val_str)
}

fn camel_case(name: &str) -> String {
    let name = name.trim_start_matches('_');
    let is_screaming = name.chars().any(|c| c.is_alphabetic())
        && name
            .chars()
            .all(|c| c.is_uppercase() || c == '_' || c == '-' || c.is_numeric());

    if is_screaming {
        let words: Vec<&str> = name
            .split(|c| c == '_' || c == '-')
            .filter(|w| !w.is_empty())
            .collect();
        let mut out = String::new();
        for (i, word) in words.iter().enumerate() {
            let lower = word.to_lowercase();
            if i == 0 {
                out.push_str(&lower);
            } else {
                let mut chars = lower.chars();
                if let Some(first) = chars.next() {
                    out.extend(first.to_uppercase());
                    out.extend(chars);
                }
            }
        }
        return out;
    }

    let mut out = String::with_capacity(name.len());
    let mut uppercase_next = false;
    for (i, ch) in name.chars().enumerate() {
        if ch == '_' || ch == '-' {
            uppercase_next = true;
            continue;
        }
        if i == 0 {
            out.extend(ch.to_lowercase());
        } else if uppercase_next {
            out.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn collect_program_pdas(idl: &Idl) -> Vec<Value> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut pdas: Vec<Value> = Vec::new();

    for instruction in &idl.instructions {
        let args = instruction.args.as_deref().unwrap_or(&[]);
        for account in &instruction.accounts {
            if let Some(pda) = &account.pda {
                let name = account.name.clone();
                if seen.insert(name.clone()) {
                    pdas.push(codama_pda_node(&name, &pda.seeds, args));
                }
            }
        }
    }

    pdas
}

fn codama_pda_node(name: &str, seeds: &[IdlPdaSeed], args: &[IdlArg]) -> Value {
    json!({
        "kind": "pdaNode",
        "name": camel_case(name),
        "seeds": seeds.iter().map(|s| codama_pda_seed(s, args)).collect::<Vec<_>>(),
    })
}

fn codama_pda_seed(seed: &IdlPdaSeed, args: &[IdlArg]) -> Value {
    match seed {
        IdlPdaSeed::Const { value } => {
            if let Ok(s) = std::str::from_utf8(value) {
                json!({
                    "kind": "constantPdaSeedNode",
                    "type": { "kind": "stringTypeNode", "encoding": "utf8" },
                    "value": { "kind": "stringValueNode", "string": s },
                })
            } else {
                json!({
                    "kind": "constantPdaSeedNode",
                    "type": fixed_bytes_type(value.len()),
                    "value": {
                        "kind": "bytesValueNode",
                        "data": hex_bytes(value),
                        "encoding": "base16",
                    },
                })
            }
        }
        // account seeds are always pubkeys.
        IdlPdaSeed::Account { path, account } => {
            let mut node = json!({
                "kind": "variablePdaSeedNode",
                "name": camel_case(path),
                "type": { "kind": "publicKeyTypeNode" },
            });
            if let Some(acc) = account {
                node["account"] = json!(camel_case(acc));
            }
            node
        }

        // default to `bytes` so if no arg.
        IdlPdaSeed::Arg { path } => {
            let resolved_type = args
                .iter()
                .find(|a| a.name == *path)
                .map(|a| codama_field_type(&a.r#type))
                .unwrap_or_else(|| json!({ "kind": "bytesTypeNode" }));

            json!({
                "kind": "variablePdaSeedNode",
                "name": camel_case(path),
                "type": resolved_type,
            })
        }
    }
}
