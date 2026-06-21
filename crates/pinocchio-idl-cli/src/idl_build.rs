use std::{fs, path::Path};
use syn::{Fields, ItemStruct};

use pinocchio_idl_core::{
    account_discriminator, derive_instruction_name, find_accounts_param, rust_type_to_idl_type,
    Idl, IdlAccountDef, IdlField, IdlType, IdlTypeDefinition, Instruction, Metadata,
};

use crate::discover::discover;

fn state_to_idl(item: &ItemStruct) -> syn::Result<(IdlAccountDef, IdlTypeDefinition)> {
    let name = item.ident.to_string();

    let fields = match &item.fields {
        Fields::Named(named) => named.named.iter()
            .map(|f| {
                let field_name = f.ident.as_ref()
                    .ok_or_else(|| syn::Error::new_spanned(f, "state fields must be named"))?
                    .to_string();
                Ok(IdlField { name: field_name, r#type: rust_type_to_idl_type(&f.ty)? })
            })
            .collect::<syn::Result<Vec<_>>>()?,
        other => return Err(syn::Error::new_spanned(other, "#[p_state] requires named fields")),
    };

    Ok((
        IdlAccountDef { name: name.clone(), discriminator: account_discriminator(&name).to_vec() },
        IdlTypeDefinition { name, r#type: IdlType { kind: "struct".to_string(), fields } },
    ))
}

pub fn build_idl(src_dir: &Path, metadata: Metadata) -> syn::Result<Idl> {
    let discovery = discover(src_dir)?;

    let instructions = discovery.instructions.iter()
        .enumerate()
        .map(|(index, discovered)| {
            let mut instruction: Instruction = syn::parse2(discovered.attr_tokens.clone())?;
            let accounts_ident = find_accounts_param(&discovered.func.sig)?;
            instruction.add_accounts(&discovered.func.block.stmts, &accounts_ident.to_string());
            let name = derive_instruction_name(&discovered.func.sig.ident);
            instruction.into_idl(name, index as u8)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let (accounts, types): (Vec<_>, Vec<_>) = discovery.states.iter()
        .map(state_to_idl)
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .unzip();

    Ok(Idl {
        address: discovery.program_id.unwrap_or_default(),
        metadata, instructions, accounts,
        errors: Vec::new(),
        types,
        constants: Vec::new(),
    })
}

pub fn write_idl(idl: &Idl, out_path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(idl).expect("Idl serialization is infallible");
    fs::write(out_path, json)
}