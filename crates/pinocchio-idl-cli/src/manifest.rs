use serde::Deserialize;
use std::path::Path;
use std::fs;
use pinocchio_idl_core::Metadata;
#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackage,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
    description: Option<String>,
}

pub fn read_metadata(manifest_path: &Path) -> syn::Result<Metadata> {
    let content = fs::read_to_string(manifest_path).map_err(|e| {
        
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("reading {}: {e}", manifest_path.display()),
        )

    })?;

    let parsed: CargoToml = toml::from_str(&content).map_err(|e| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("parsing {}: {e}", manifest_path.display()),
        )
    })?;

    Ok(Metadata {
        name: parsed.package.name,
        version: parsed.package.version,
        spec: "0.1.0".to_string(),
        description: parsed.package.description
            .unwrap_or_else(|| "Created with PinIDL".to_string()),
    })
}