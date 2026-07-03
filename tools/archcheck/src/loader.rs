use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("missing section: {0}")]
    MissingSection(&'static str),
    #[error("dev-dependencies-audit.enabled is missing; set it to true or false explicitly")]
    MissingDevDepAuditFlag,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Architecture {
    pub crate_layers: HashMap<String, usize>,
    pub forbidden_deps: HashMap<String, String>,
    pub audited_deps: HashMap<String, String>,
    pub dev_dep_audit_enabled: bool,
    pub proc_macro_exemptions: HashMap<String, ProcMacroExemption>,
    pub reverse_dependency_exemptions: HashMap<String, ProcMacroExemption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcMacroExemption {
    pub allowed_deps: Vec<String>,
    pub reason: String,
}

impl Architecture {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, LoaderError> {
        let content = fs::read_to_string(path.as_ref())?;
        Self::from_toml_str(&content)
    }

    pub fn from_toml_str(toml_str: &str) -> Result<Self, LoaderError> {
        let parsed: toml::Value = toml::from_str(toml_str)?;

        let crate_layers = parsed
            .get("crate-layers")
            .and_then(|v| v.as_table())
            .map(|table| {
                table
                    .iter()
                    .filter_map(|(k, v)| v.as_integer().map(|i| (k.clone(), i as usize)))
                    .collect()
            })
            .ok_or_else(|| LoaderError::MissingSection("crate-layers"))?;

        let forbidden_deps = Self::parse_reason_map(parsed.get("forbidden-deps"))?;
        let audited_deps = Self::parse_reason_map(parsed.get("audited-deps"))?;

        let dev_dep_audit_enabled = parsed
            .get("dev-dependencies-audit")
            .and_then(|v| v.get("enabled"))
            .and_then(|v| v.as_bool())
            .ok_or_else(|| LoaderError::MissingDevDepAuditFlag)?;

        let proc_macro_exemptions = Self::parse_exemptions(parsed.get("proc-macro-exemptions"))?;
        let reverse_dependency_exemptions =
            Self::parse_exemptions(parsed.get("reverse-dependency-exemptions"))?;

        Ok(Architecture {
            crate_layers,
            forbidden_deps,
            audited_deps,
            dev_dep_audit_enabled,
            proc_macro_exemptions,
            reverse_dependency_exemptions,
        })
    }

    fn parse_reason_map(value: Option<&toml::Value>) -> Result<HashMap<String, String>, LoaderError> {
        let table = match value {
            Some(toml::Value::Table(t)) => t,
            _ => return Ok(HashMap::new()),
        };
        let mut map = HashMap::new();
        for (k, v) in table {
            let reason = v.as_str().unwrap_or("").to_string();
            map.insert(k.clone(), reason);
        }
        Ok(map)
    }

    fn parse_exemptions(
        value: Option<&toml::Value>,
    ) -> Result<HashMap<String, ProcMacroExemption>, LoaderError> {
        let table = match value {
            Some(toml::Value::Table(t)) => t,
            _ => return Ok(HashMap::new()),
        };
        let mut map = HashMap::new();
        for (k, v) in table {
            let allowed = v
                .get("allowed_deps")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let reason = v.get("reason").and_then(|x| x.as_str()).unwrap_or("").to_string();
            map.insert(k.clone(), ProcMacroExemption { allowed_deps: allowed, reason });
        }
        Ok(map)
    }
}
