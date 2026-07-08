use crate::{visit_items, walk_rs_files};
use std::{fs, path::Path};
use syn::{ItemConst, ItemEnum, ItemFn, ItemStruct};

pub struct DiscoveredInstruction {
    pub func: ItemFn,
    pub attr_tokens: proc_macro2::TokenStream,
}

#[derive(Default)]
pub struct Discovery {
    pub instructions: Vec<DiscoveredInstruction>,
    pub states: Vec<ItemStruct>,
    pub errors: Vec<ItemEnum>,
    pub constants: Vec<ItemConst>,
    pub program_id: Option<String>,
}

pub fn discover(src_dir: &Path) -> syn::Result<Discovery> {
    let mut discovery = Discovery::default();

    for path in walk_rs_files(src_dir) {
        let content = fs::read_to_string(&path).map_err(|e| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("reading {}: {e}", path.display()),
            )
        })?;

        let file = syn::parse_file(&content).map_err(|e| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("parsing {}: {e}", path.display()),
            )
        })?;
        visit_items(&file.items, &mut discovery);
    }

    Ok(discovery)
}
