//! Template registry with version management.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::PromptError;
use crate::template::PromptTemplate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRegistry {
    templates: HashMap<String, HashMap<String, PromptTemplate>>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self { templates: HashMap::new() }
    }

    pub fn register(&mut self, template: PromptTemplate) -> Result<(), PromptError> {
        let versions = self.templates.entry(template.name.clone()).or_default();

        if versions.contains_key(&template.version) {
            return Err(PromptError::VersionConflict(format!(
                "template '{}' version '{}' already exists",
                template.name, template.version
            )));
        }

        versions.insert(template.version.clone(), template);
        Ok(())
    }

    pub fn get(&self, name: &str, version: Option<&str>) -> Option<&PromptTemplate> {
        let versions = self.templates.get(name)?;

        match version {
            Some(v) => versions.get(v),
            None => {
                let mut latest: Option<&PromptTemplate> = None;
                for template in versions.values() {
                    if latest.is_none()
                        || compare_versions(
                            &template.version,
                            latest.map(|l| l.version.as_str()).unwrap_or(""),
                        ) == std::cmp::Ordering::Greater
                    {
                        latest = Some(template);
                    }
                }
                latest
            }
        }
    }

    pub fn get_latest(&self, name: &str) -> Option<&PromptTemplate> {
        self.get(name, None)
    }

    pub fn get_version(&self, name: &str, version: &str) -> Option<&PromptTemplate> {
        self.get(name, Some(version))
    }

    pub fn has_template(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    pub fn has_version(&self, name: &str, version: &str) -> bool {
        self.templates.get(name).map(|v| v.contains_key(version)).unwrap_or(false)
    }

    pub fn list_templates(&self) -> Vec<String> {
        let mut names: Vec<String> = self.templates.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn list_versions(&self, name: &str) -> Vec<String> {
        let mut versions: Vec<String> =
            self.templates.get(name).map(|v| v.keys().cloned().collect()).unwrap_or_default();
        versions.sort_by(|a, b| compare_versions(b, a));
        versions
    }

    pub fn render(
        &self,
        name: &str,
        version: Option<&str>,
        values: &HashMap<String, serde_json::Value>,
    ) -> Result<String, PromptError> {
        let template =
            self.get(name, version).ok_or_else(|| PromptError::NotFound(name.to_string()))?;
        template.render(values)
    }

    pub fn render_latest(
        &self,
        name: &str,
        values: &HashMap<String, serde_json::Value>,
    ) -> Result<String, PromptError> {
        self.render(name, None, values)
    }

    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    pub fn total_versions(&self) -> usize {
        self.templates.values().map(|v| v.len()).sum()
    }

    pub fn remove(&mut self, name: &str) -> bool {
        self.templates.remove(name).is_some()
    }

    pub fn remove_version(&mut self, name: &str, version: &str) -> bool {
        if let Some(versions) = self.templates.get_mut(name) {
            versions.remove(version).is_some()
        } else {
            false
        }
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parts_a: Vec<&str> = a.split('.').collect();
    let parts_b: Vec<&str> = b.split('.').collect();

    for i in 0..parts_a.len().max(parts_b.len()) {
        let num_a = parts_a.get(i).and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);
        let num_b = parts_b.get(i).and_then(|p| p.parse::<u64>().ok()).unwrap_or(0);

        match num_a.cmp(&num_b) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    std::cmp::Ordering::Equal
}
