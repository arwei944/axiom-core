//! Prompt template implementation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::PromptError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    String,
    Number,
    Boolean,
    List,
    Object,
}

impl VariableType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VariableType::String => "string",
            VariableType::Number => "number",
            VariableType::Boolean => "boolean",
            VariableType::List => "list",
            VariableType::Object => "object",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: VariableType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
}

impl TemplateVariable {
    pub fn new(name: impl Into<String>, var_type: VariableType) -> Self {
        Self {
            name: name.into(),
            var_type,
            required: true,
            default: None,
            description: None,
        }
    }

    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self.required = false;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn validate(&self, value: &serde_json::Value) -> Result<(), PromptError> {
        let type_matches = match self.var_type {
            VariableType::String => value.is_string(),
            VariableType::Number => value.is_number(),
            VariableType::Boolean => value.is_boolean(),
            VariableType::List => value.is_array(),
            VariableType::Object => value.is_object(),
        };

        if !type_matches {
            return Err(PromptError::InvalidType(format!(
                "variable '{}' expected type {}, got {}",
                self.name,
                self.var_type.as_str(),
                value_type_name(value)
            )));
        }

        Ok(())
    }
}

fn value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Array(_) => "list",
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Null => "null",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub template: String,
    pub variables: Vec<TemplateVariable>,
    pub sections: Vec<String>,
}

impl PromptTemplate {
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: "1.0.0".to_string(),
            description: None,
            template: template.into(),
            variables: Vec::new(),
            sections: Vec::new(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_variable(mut self, var: TemplateVariable) -> Self {
        self.variables.push(var);
        self
    }

    pub fn with_section(mut self, section: impl Into<String>) -> Self {
        self.sections.push(section.into());
        self
    }

    pub fn validate_variables(
        &self,
        values: &HashMap<String, serde_json::Value>,
    ) -> Result<(), PromptError> {
        for var in &self.variables {
            match values.get(&var.name) {
                Some(value) => {
                    var.validate(value)?;
                }
                None => {
                    if var.required && var.default.is_none() {
                        return Err(PromptError::MissingVariable(var.name.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn render(
        &self,
        values: &HashMap<String, serde_json::Value>,
    ) -> Result<String, PromptError> {
        self.validate_variables(values)?;

        let mut result = self.template.clone();

        for var in &self.variables {
            let value = match values.get(&var.name) {
                Some(v) => v.clone(),
                None => var.default.clone().unwrap_or(serde_json::Value::Null),
            };

            let placeholder = format!("{{{{{}}}}}", var.name);
            let value_str = value_to_string(&value);
            result = result.replace(&placeholder, &value_str);
        }

        Ok(result)
    }

    pub fn render_with_defaults(&self) -> Result<String, PromptError> {
        let mut values = HashMap::new();
        for var in &self.variables {
            if let Some(default) = &var.default {
                values.insert(var.name.clone(), default.clone());
            }
        }
        self.render(&values)
    }

    pub fn variable_names(&self) -> Vec<&str> {
        self.variables.iter().map(|v| v.name.as_str()).collect()
    }

    pub fn required_variables(&self) -> Vec<&TemplateVariable> {
        self.variables.iter().filter(|v| v.required).collect()
    }

    pub fn optional_variables(&self) -> Vec<&TemplateVariable> {
        self.variables.iter().filter(|v| !v.required).collect()
    }

    pub fn compose(
        &self,
        other: &PromptTemplate,
        section_name: &str,
    ) -> Result<PromptTemplate, PromptError> {
        let placeholder = format!("{{%{}%}}", section_name);

        if !self.template.contains(&placeholder) {
            return Err(PromptError::RenderError(format!(
                "section '{}' not found in template '{}'",
                section_name, self.name
            )));
        }

        let mut new_template = self.template.clone();
        new_template = new_template.replace(&placeholder, &other.template);

        let mut variables = self.variables.clone();
        for var in &other.variables {
            if !variables.iter().any(|v| v.name == var.name) {
                variables.push(var.clone());
            }
        }

        let mut sections = self.sections.clone();
        sections.extend(other.sections.clone());

        Ok(PromptTemplate {
            name: format!("{}_{}", self.name, other.name),
            version: self.version.clone(),
            description: self.description.clone(),
            template: new_template,
            variables,
            sections,
        })
    }
}

fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_string).collect();
            items.join(", ")
        }
        serde_json::Value::Object(obj) => {
            let pairs: Vec<String> = obj
                .iter()
                .map(|(k, v)| format!("{}: {}", k, value_to_string(v)))
                .collect();
            pairs.join("\n")
        }
        serde_json::Value::Null => "".to_string(),
    }
}
