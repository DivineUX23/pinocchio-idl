pub mod discover;
pub mod idl_build;
pub mod manifest;

pub use discover::{discover, Discovery, DiscoveredInstruction};
pub use idl_build::{build_idl, write_idl};
pub use manifest::read_metadata;

use std::path::{Path, PathBuf};
use std::fs;
use syn::Item;

pub(crate) fn walk_rs_files(dir: &Path) -> Vec<PathBuf> {
    // Scans every .rs file under src/ directly, rather than resolving the
    // `mod foo;` tree properly — simpler, and it already covers multi-file
    // programs since every file gets parsed independently regardless of how
    // it's wired into the module tree.
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return files };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_rs_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}

pub(crate) fn visit_items(items: &[Item], discovery: &mut Discovery) {
    for item in items {
        match item {
            Item::Fn(func) => {
                if let Some(attr) = find_attr(&func.attrs, "p_instruction") {
                    discovery.instructions.push(DiscoveredInstruction {
                        func: func.clone(),
                        attr_tokens: attr_tokens(attr),
                    });
                }
            }
            Item::Struct(s) => {
                if find_attr(&s.attrs, "p_state").is_some() {
                    discovery.states.push(s.clone());
                }
            }
            Item::Macro(mac) => {
                // declare_id!("...") — parse the string literal out of the body.
                if mac.mac.path.is_ident("declare_id") {
                    if let Ok(lit) = mac.mac.parse_body::<syn::LitStr>() {
                        discovery.program_id = Some(lit.value());
                    }
                }
            }
            Item::Mod(m) => {
                // only matters for inline `mod foo { .. }` bodies — `mod foo;`
                // pointing at another file is already covered by the file walk.
                if let Some((_, inner)) = &m.content {
                    visit_items(inner, discovery);
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn find_attr<'a>(attrs: &'a [syn::Attribute], name: &str) -> Option<&'a syn::Attribute> {
    attrs.iter().find(|a| a.path().is_ident(name))
}

pub(crate) fn attr_tokens(attr: &syn::Attribute) -> proc_macro2::TokenStream {
    match &attr.meta {
        syn::Meta::List(list) => list.tokens.clone(),
        _ => proc_macro2::TokenStream::new(),
    }
}