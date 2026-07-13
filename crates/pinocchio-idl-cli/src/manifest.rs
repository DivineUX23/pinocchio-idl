use pinocchio_idl_core::Metadata;
use serde::Deserialize;
use std::fs;
use std::path::Path;
#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackage,
    lib: Option<CargoLib>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct CargoLib {
    path: Option<String>,
}

pub fn read_metadata(manifest_path: &Path) -> syn::Result<(Metadata, Option<std::path::PathBuf>)> {
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

    let lib_path = parsed.lib.and_then(|l| l.path).map(std::path::PathBuf::from);

    Ok((
        Metadata {
            name: parsed.package.name,
            version: parsed.package.version,
            spec: "0.1.0".to_string(),
            description: parsed
                .package
                .description
                .unwrap_or_else(|| "Created with Pinocchio-IDL".to_string()),
        },
        lib_path,
    ))
}
