use crate::{find_attr, walk_rs_files};
use anyhow::{Context, Result};
use std::{collections::HashSet, fs, path::Path};
use syn::Item;

#[derive(serde::Serialize)]
pub struct DoctorFinding {
    pub file: String,
    pub line: usize,
    pub severity: String,
    pub message: String,
}

#[derive(serde::Serialize, Default)]
pub struct DoctorReport {
    pub findings: Vec<DoctorFinding>,
}

pub fn run_doctor(src_dir: &Path) -> Result<DoctorReport> {
    let files = walk_rs_files(src_dir);
    let mut report = DoctorReport::default();
    let mut annotated_instructions = HashSet::new();

    for path in &files {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Disk I/O failed while reading {}", path.display()))?;

        let file = syn::parse_file(&content)
            .map_err(|e| anyhow::anyhow!("AST Parse Error in {}: {e}", path.display()))?;

        let file_display = path.display().to_string();

        for item in &file.items {
            match item {
                Item::Fn(func) => {
                    let name = func.sig.ident.to_string();
                    let line = func.sig.ident.span().start().line;

                    if let Some(_attr) = find_attr(&func.attrs, "p_instruction") {
                        if !annotated_instructions.insert(name.clone()) {
                            report.findings.push(DoctorFinding {
                                file: file_display.clone(),
                                line,
                                severity: "warning".to_string(),
                                message: format!("duplicate instruction name `{name}`"),
                            });
                        }
                    } else if name.starts_with("process_")
                        || name == "process"
                        || name.contains("instruction")
                    {
                        report.findings.push(DoctorFinding {
                            file: file_display.clone(),
                            line,
                            severity: "warning".to_string(),
                            message: format!(
                                "function `{name}` looks like an instruction processor but has no #[p_instruction]"
                            ),
                        });
                    }
                }
                Item::Struct(s) => {
                    let name = s.ident.to_string();
                    let line = s.ident.span().start().line;

                    let looks_like_state = name.ends_with("State")
                        || name.ends_with("Account")
                        || name.ends_with("Config");

                    if looks_like_state && find_attr(&s.attrs, "p_state").is_none() {
                        report.findings.push(DoctorFinding {
                            file: file_display.clone(),
                            line,
                            severity: "warning".to_string(),
                            message: format!(
                                "struct `{name}` looks like state but has no #[p_state]"
                            ),
                        });
                    }
                }
                Item::Enum(e) => {
                    let name = e.ident.to_string();
                    let line = e.ident.span().start().line;

                    if name.ends_with("Error") && find_attr(&e.attrs, "p_error").is_none() {
                        report.findings.push(DoctorFinding {
                            file: file_display.clone(),
                            line,
                            severity: "warning".to_string(),
                            message: format!(
                                "enum `{name}` looks like an error but has no #[p_error]"
                            ),
                        });
                    }
                }
                Item::Const(c) => {
                    let name = c.ident.to_string();
                    let line = c.ident.span().start().line;

                    let is_pub = matches!(c.vis, syn::Visibility::Public(_));
                    if is_pub && find_attr(&c.attrs, "p_constant").is_none() {
                        report.findings.push(DoctorFinding {
                            file: file_display.clone(),
                            line,
                            severity: "warning".to_string(),
                            message: format!(
                                "constant `{name}` is public but has no #[p_constant]"
                            ),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(report)
}
