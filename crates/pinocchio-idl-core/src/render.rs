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
                .zip(idl.types.iter())
                .map(|(acc, ty)| codama_account(acc, ty))
                .collect::<Vec<_>>(),
            "instructions": idl.instructions.iter().map(codama_instruction).collect::<Vec<_>>(),

            "definedTypes": idl.types.iter().map(|ty| json!({
                "kind": "definedTypeNode",
                "name": camel_case(&ty.name),
                "type": {
                    "kind": "structTypeNode",
                    "fields": ty.r#type.fields.iter().map(codama_struct_field).collect::<Vec<_>>(),
                },
            })).collect::<Vec<_>>(),

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
                    "number": c.value,
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

fn codama_account(account: &crate::IdlAccountDef, ty: &IdlTypeDefinition) -> Value {
    let discriminator_bytes = &account.discriminator;

    let fields: Vec<Value> = std::iter::once(codama_discriminator_field(discriminator_bytes))
        .chain(ty.r#type.fields.iter().map(codama_struct_field))
        .collect();

    json!({
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
    })
}

fn codama_instruction(instruction: &IdlInstruction) -> Value {
    let discriminator_bytes = &instruction.discriminator;
    let args = instruction.args.as_deref().unwrap_or(&[]);
    let mut arguments = vec![codama_discriminator_argument(discriminator_bytes)];
    arguments.extend(args.iter().map(codama_instruction_argument));

    json!({
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
    })
}

fn codama_instruction_account(account: &IdlAccount, _args: &[IdlArg]) -> Value {
    let mut value = json!({
        "kind": "instructionAccountNode",
        "name": camel_case(&account.name),
        "isWritable": account.writable,
        "isSigner": account.signer,
        "isOptional": false,
    });

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
    json!({
        "kind": "structFieldTypeNode",
        "name": camel_case(&field.name),
        "type": codama_field_type(&field.r#type),
    })
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

fn camel_case(name: &str) -> String {
    let name = name.trim_start_matches('_');
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
