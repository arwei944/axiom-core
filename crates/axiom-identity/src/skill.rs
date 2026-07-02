//! Skill system for agent capabilities.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillState {
    Inactive,
    Active,
    Cooldown,
    Disabled,
}

impl SkillState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillState::Inactive => "inactive",
            SkillState::Active => "active",
            SkillState::Cooldown => "cooldown",
            SkillState::Disabled => "disabled",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, SkillState::Active)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivationCondition {
    Always,
    Never,
    KeywordTrigger(Vec<String>),
    ContextMatch(String),
    UserRequest,
    Schedule(String),
    And(Vec<ActivationCondition>),
    Or(Vec<ActivationCondition>),
    Not(Box<ActivationCondition>),
}

impl ActivationCondition {
    pub fn evaluate(&self, context: &SkillContext) -> bool {
        match self {
            ActivationCondition::Always => true,
            ActivationCondition::Never => false,
            ActivationCondition::KeywordTrigger(keywords) => {
                let context_text = context.text.to_lowercase();
                keywords.iter().any(|k| context_text.contains(&k.to_lowercase()))
            }
            ActivationCondition::ContextMatch(pattern) => {
                context.text.to_lowercase().contains(&pattern.to_lowercase())
            }
            ActivationCondition::UserRequest => context.user_requested,
            ActivationCondition::Schedule(_schedule) => {
                false
            }
            ActivationCondition::And(conditions) => {
                conditions.iter().all(|c| c.evaluate(context))
            }
            ActivationCondition::Or(conditions) => {
                conditions.iter().any(|c| c.evaluate(context))
            }
            ActivationCondition::Not(condition) => !condition.evaluate(context),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillContext<'a> {
    pub text: &'a str,
    pub user_requested: bool,
    pub current_time: u64,
    pub active_skills: &'a [String],
}

impl<'a> SkillContext<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            user_requested: false,
            current_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            active_skills: &[],
        }
    }

    pub fn with_user_requested(mut self, requested: bool) -> Self {
        self.user_requested = requested;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub state: SkillState,
    pub activation: ActivationCondition,
    pub tools: Vec<String>,
    pub prompt_fragments: Vec<String>,
    pub priority: u32,
    pub cooldown_seconds: u64,
    pub last_activated: Option<u64>,
    pub metadata: serde_json::Value,
}

impl Skill {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            state: SkillState::Inactive,
            activation: ActivationCondition::UserRequest,
            tools: Vec::new(),
            prompt_fragments: Vec::new(),
            priority: 0,
            cooldown_seconds: 0,
            last_activated: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_activation(mut self, condition: ActivationCondition) -> Self {
        self.activation = condition;
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_prompt_fragments(mut self, fragments: Vec<String>) -> Self {
        self.prompt_fragments = fragments;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_cooldown(mut self, seconds: u64) -> Self {
        self.cooldown_seconds = seconds;
        self
    }

    pub fn with_metadata(mut self, meta: serde_json::Value) -> Self {
        self.metadata = meta;
        self
    }

    pub fn can_activate(&self, context: &SkillContext) -> bool {
        if self.state == SkillState::Disabled {
            return false;
        }

        if let Some(last) = self.last_activated {
            let now = context.current_time;
            if now.saturating_sub(last) < self.cooldown_seconds {
                return false;
            }
        }

        self.activation.evaluate(context)
    }

    pub fn activate(&mut self, context: &SkillContext) -> bool {
        if self.can_activate(context) {
            self.state = SkillState::Active;
            self.last_activated = Some(context.current_time);
            true
        } else {
            false
        }
    }

    pub fn deactivate(&mut self) {
        if self.state == SkillState::Active {
            self.state = SkillState::Inactive;
        }
    }

    pub fn disable(&mut self) {
        self.state = SkillState::Disabled;
    }

    pub fn enable(&mut self) {
        if self.state == SkillState::Disabled {
            self.state = SkillState::Inactive;
        }
    }
}