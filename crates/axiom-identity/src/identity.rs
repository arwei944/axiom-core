//! Agent identity definition.

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, Clone)]
pub enum IdentityError {
    #[error("identity not found: {0}")]
    NotFound(String),
    #[error("invalid identity: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub traits: Vec<String>,
    pub capabilities: Vec<String>,
    pub tone: String,
    pub disclosure_level: DisclosureLevel,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisclosureLevel {
    Minimal,
    Basic,
    Full,
    Transparent,
}

impl DisclosureLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisclosureLevel::Minimal => "minimal",
            DisclosureLevel::Basic => "basic",
            DisclosureLevel::Full => "full",
            DisclosureLevel::Transparent => "transparent",
        }
    }

    pub fn order(&self) -> u8 {
        match self {
            DisclosureLevel::Minimal => 0,
            DisclosureLevel::Basic => 1,
            DisclosureLevel::Full => 2,
            DisclosureLevel::Transparent => 3,
        }
    }

    pub fn can_disclose(&self, required: DisclosureLevel) -> bool {
        self.order() >= required.order()
    }
}

impl AgentIdentity {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            system_prompt: String::new(),
            traits: Vec::new(),
            capabilities: Vec::new(),
            tone: "professional".to_string(),
            disclosure_level: DisclosureLevel::Basic,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    pub fn with_traits(mut self, traits: Vec<String>) -> Self {
        self.traits = traits;
        self
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn with_tone(mut self, tone: impl Into<String>) -> Self {
        self.tone = tone.into();
        self
    }

    pub fn with_disclosure_level(mut self, level: DisclosureLevel) -> Self {
        self.disclosure_level = level;
        self
    }

    pub fn with_metadata(mut self, meta: serde_json::Value) -> Self {
        self.metadata = meta;
        self
    }

    pub fn persona_summary(&self, level: DisclosureLevel) -> String {
        let mut parts = Vec::new();

        if level.can_disclose(DisclosureLevel::Minimal) {
            parts.push(format!("Name: {}", self.name));
        }

        if level.can_disclose(DisclosureLevel::Basic) {
            if !self.description.is_empty() {
                parts.push(format!("Role: {}", self.description));
            }
            parts.push(format!("Tone: {}", self.tone));
        }

        if level.can_disclose(DisclosureLevel::Full) {
            if !self.traits.is_empty() {
                parts.push(format!("Traits: {}", self.traits.join(", ")));
            }
            if !self.capabilities.is_empty() {
                parts.push(format!("Capabilities: {}", self.capabilities.join(", ")));
            }
        }

        if level.can_disclose(DisclosureLevel::Transparent) {
            parts.push(format!("Identity ID: {}", self.id));
            parts.push(format!(
                "Disclosure Level: {}",
                self.disclosure_level.as_str()
            ));
        }

        parts.join("\n")
    }

    pub fn build_system_prompt(&self) -> String {
        let mut prompt = String::new();

        if !self.system_prompt.is_empty() {
            prompt.push_str(&self.system_prompt);
            prompt.push_str("\n\n");
        }

        prompt.push_str(&format!("Your name is {}. ", self.name));

        if !self.description.is_empty() {
            prompt.push_str(&format!("{}\n\n", self.description));
        }

        if !self.traits.is_empty() {
            prompt.push_str("Core traits:\n");
            for trait_ in &self.traits {
                prompt.push_str(&format!("- {}\n", trait_));
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!("Communication style: {} tone.\n", self.tone));

        prompt
    }
}
