//! Agent persona - combined identity and skills.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::identity::{AgentIdentity, DisclosureLevel, IdentityError};
use crate::skill::{Skill, SkillContext, SkillState};

pub struct AgentPersona {
    identity: Arc<RwLock<AgentIdentity>>,
    skills: Arc<RwLock<HashMap<String, Skill>>>,
    active_skills: Arc<RwLock<Vec<String>>>,
}

impl AgentPersona {
    pub fn new(identity: AgentIdentity) -> Self {
        Self {
            identity: Arc::new(RwLock::new(identity)),
            skills: Arc::new(RwLock::new(HashMap::new())),
            active_skills: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn identity(&self) -> AgentIdentity {
        self.identity.read().clone()
    }

    pub fn set_identity(&self, identity: AgentIdentity) {
        *self.identity.write() = identity;
    }

    pub fn add_skill(&self, skill: Skill) {
        let id = skill.id.clone();
        self.skills.write().insert(id, skill);
    }

    pub fn remove_skill(&self, skill_id: &str) -> bool {
        self.skills.write().remove(skill_id).is_some()
    }

    pub fn get_skill(&self, skill_id: &str) -> Option<Skill> {
        self.skills.read().get(skill_id).cloned()
    }

    pub fn list_skills(&self) -> Vec<Skill> {
        let mut skills: Vec<Skill> = self.skills.read().values().cloned().collect();
        skills.sort_by_key(|b| std::cmp::Reverse(b.priority));
        skills
    }

    pub fn active_skills(&self) -> Vec<Skill> {
        let active = self.active_skills.read();
        let skills = self.skills.read();
        active
            .iter()
            .filter_map(|id| skills.get(id).cloned())
            .collect()
    }

    pub fn update_skills_for_context(&self, text: &str, user_requested: bool) -> Vec<String> {
        let mut newly_activated = Vec::new();

        let active_ids = self.active_skills.read().clone();
        let context = SkillContext {
            text,
            user_requested,
            current_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            active_skills: &active_ids,
        };

        let mut skills = self.skills.write();
        let mut active = self.active_skills.write();

        for (id, skill) in skills.iter_mut() {
            if skill.state == SkillState::Active {
                if !skill.activation.evaluate(&context) {
                    skill.deactivate();
                    active.retain(|s| s != id);
                }
            } else if skill.can_activate(&context) && skill.activate(&context) {
                newly_activated.push(id.clone());
                active.push(id.clone());
            }
        }

        newly_activated
    }

    pub fn activate_skill(&self, skill_id: &str) -> Result<bool, IdentityError> {
        let mut skills = self.skills.write();
        let skill = skills
            .get_mut(skill_id)
            .ok_or_else(|| IdentityError::NotFound(skill_id.to_string()))?;

        let context = SkillContext::new("").with_user_requested(true);
        let activated = skill.activate(&context);

        if activated {
            let mut active = self.active_skills.write();
            if !active.contains(&skill_id.to_string()) {
                active.push(skill_id.to_string());
            }
        }

        Ok(activated)
    }

    pub fn deactivate_skill(&self, skill_id: &str) -> Result<bool, IdentityError> {
        let mut skills = self.skills.write();
        let skill = skills
            .get_mut(skill_id)
            .ok_or_else(|| IdentityError::NotFound(skill_id.to_string()))?;

        skill.deactivate();

        let mut active = self.active_skills.write();
        let was_active = active.iter().position(|s| s == skill_id).is_some();
        active.retain(|s| s != skill_id);

        Ok(was_active)
    }

    pub fn build_prompt(&self, level: DisclosureLevel) -> String {
        let identity = self.identity.read();
        let mut prompt = identity.build_system_prompt();

        let active = self.active_skills();
        if !active.is_empty() {
            prompt.push_str("\nActive capabilities:\n");
            for skill in &active {
                prompt.push_str(&format!("- {}: {}\n", skill.name, skill.description));
                for fragment in &skill.prompt_fragments {
                    prompt.push_str(&format!("  {}\n", fragment));
                }
            }
        }

        if level.can_disclose(DisclosureLevel::Full) {
            let all_skills = self.list_skills();
            if !all_skills.is_empty() {
                prompt.push_str("\nAvailable capabilities:\n");
                for skill in &all_skills {
                    if skill.state != SkillState::Disabled {
                        prompt.push_str(&format!("- {} (priority: {})\n", skill.name, skill.priority));
                    }
                }
            }
        }

        prompt
    }

    pub fn available_tools(&self) -> Vec<String> {
        let active = self.active_skills();
        let mut tools = Vec::new();
        for skill in &active {
            for tool in &skill.tools {
                if !tools.contains(tool) {
                    tools.push(tool.clone());
                }
            }
        }
        tools
    }

    pub fn skill_count(&self) -> usize {
        self.skills.read().len()
    }

    pub fn active_skill_count(&self) -> usize {
        self.active_skills.read().len()
    }

    pub fn disclosure_level(&self) -> DisclosureLevel {
        self.identity.read().disclosure_level
    }

    pub fn set_disclosure_level(&self, level: DisclosureLevel) {
        self.identity.write().disclosure_level = level;
    }
}

impl Clone for AgentPersona {
    fn clone(&self) -> Self {
        Self {
            identity: Arc::new(RwLock::new(self.identity.read().clone())),
            skills: Arc::new(RwLock::new(self.skills.read().clone())),
            active_skills: Arc::new(RwLock::new(self.active_skills.read().clone())),
        }
    }
}