use crate::{visit_items, walk_rs_files};
use anyhow::{Context, Result};
use std::{fs, path::Path};
use syn::{ItemConst, ItemEnum, ItemFn, ItemStruct};

pub struct DiscoveredInstruction {
    pub func: ItemFn,
    pub attr_tokens: proc_macro2::TokenStream,
    pub file: std::path::PathBuf,
}

#[derive(Default)]
pub struct Discovery {
    pub instructions: Vec<DiscoveredInstruction>,
    pub states: Vec<(ItemStruct, std::path::PathBuf)>,
    pub events: Vec<(ItemStruct, std::path::PathBuf)>,
    pub errors: Vec<(ItemEnum, std::path::PathBuf)>,
    pub constants: Vec<(ItemConst, std::path::PathBuf)>,
    pub program_id: Option<String>,
}
/*
impl Discovery {
    fn merge(mut self, other: Self) -> Self {
        self.instructions.extend(other.instructions);
        self.states.extend(other.states);
        self.errors.extend(other.errors);
        self.constants.extend(other.constants);

        if other.program_id.is_some() {
            self.program_id = other.program_id;
        }
        self
    }
}
    */

pub fn discover(src_dir: &Path) -> Result<Discovery> {
    let files: Vec<_> = walk_rs_files(src_dir);

    let mut discovery = Discovery::default();

    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Disk I/O failed while reading {}", path.display()))?;

        let file = syn::parse_file(&content)
            .map_err(|e| anyhow::anyhow!("AST Parse Error in {}: {e}", path.display()))?;

        visit_items(&file.items, &mut discovery, &path);
    }

    Ok(discovery)
}

/*
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
        visit_items(&file.items, &mut discovery, &path);
    }

    Ok(discovery)
}
    */
