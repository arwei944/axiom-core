use axiom_identity::*;
use serde_json::json;

#[test]
fn test_identity_creation() {
    let identity = AgentIdentity::new("agent-1", "TestAgent")
        .with_description("A test agent")
        .with_tone("friendly")
        .with_traits(vec!["helpful".to_string(), "concise".to_string()]);

    assert_eq!(identity.id, "agent-1");
    assert_eq!(identity.name, "TestAgent");
    assert_eq!(identity.description, "A test agent");
    assert_eq!(identity.tone, "friendly");
    assert_eq!(identity.traits.len(), 2);
}

#[test]
fn test_disclosure_level_order() {
    assert!(DisclosureLevel::Transparent.can_disclose(DisclosureLevel::Full));
    assert!(DisclosureLevel::Full.can_disclose(DisclosureLevel::Basic));
    assert!(DisclosureLevel::Basic.can_disclose(DisclosureLevel::Minimal));

    assert!(!DisclosureLevel::Minimal.can_disclose(DisclosureLevel::Basic));
    assert!(!DisclosureLevel::Basic.can_disclose(DisclosureLevel::Full));
}

#[test]
fn test_disclosure_level_strings() {
    assert_eq!(DisclosureLevel::Minimal.as_str(), "minimal");
    assert_eq!(DisclosureLevel::Basic.as_str(), "basic");
    assert_eq!(DisclosureLevel::Full.as_str(), "full");
    assert_eq!(DisclosureLevel::Transparent.as_str(), "transparent");
}

#[test]
fn test_persona_summary_minimal() {
    let identity = AgentIdentity::new("id", "Alice")
        .with_description("Helper bot")
        .with_traits(vec!["smart".to_string()]);

    let summary = identity.persona_summary(DisclosureLevel::Minimal);
    assert!(summary.contains("Alice"));
    assert!(!summary.contains("Helper bot"));
    assert!(!summary.contains("smart"));
}

#[test]
fn test_persona_summary_full() {
    let identity = AgentIdentity::new("id", "Alice")
        .with_description("Helper bot")
        .with_traits(vec!["smart".to_string()])
        .with_capabilities(vec!["coding".to_string()]);

    let summary = identity.persona_summary(DisclosureLevel::Full);
    assert!(summary.contains("Alice"));
    assert!(summary.contains("Helper bot"));
    assert!(summary.contains("smart"));
    assert!(summary.contains("coding"));
}

#[test]
fn test_build_system_prompt() {
    let identity = AgentIdentity::new("id", "Bob")
        .with_description("Code assistant")
        .with_system_prompt("You are helpful.")
        .with_tone("professional");

    let prompt = identity.build_system_prompt();
    assert!(prompt.contains("Bob"));
    assert!(prompt.contains("Code assistant"));
    assert!(prompt.contains("You are helpful."));
    assert!(prompt.contains("professional"));
}

#[test]
fn test_skill_creation() {
    let skill = Skill::new("skill-1", "Coding")
        .with_description("Write code")
        .with_priority(10)
        .with_tools(vec!["execute".to_string()]);

    assert_eq!(skill.id, "skill-1");
    assert_eq!(skill.name, "Coding");
    assert_eq!(skill.priority, 10);
    assert_eq!(skill.tools.len(), 1);
}

#[test]
fn test_skill_state() {
    assert!(SkillState::Active.is_active());
    assert!(!SkillState::Inactive.is_active());
    assert!(!SkillState::Disabled.is_active());
    assert!(!SkillState::Cooldown.is_active());
}

#[test]
fn test_skill_state_strings() {
    assert_eq!(SkillState::Inactive.as_str(), "inactive");
    assert_eq!(SkillState::Active.as_str(), "active");
    assert_eq!(SkillState::Cooldown.as_str(), "cooldown");
    assert_eq!(SkillState::Disabled.as_str(), "disabled");
}

#[test]
fn test_activation_condition_always() {
    let context = skill::SkillContext::new("test");
    assert!(ActivationCondition::Always.evaluate(&context));
}

#[test]
fn test_activation_condition_never() {
    let context = skill::SkillContext::new("test");
    assert!(!ActivationCondition::Never.evaluate(&context));
}

#[test]
fn test_activation_condition_keyword() {
    let context = skill::SkillContext::new("I need to code something");
    let condition =
        ActivationCondition::KeywordTrigger(vec!["code".to_string(), "program".to_string()]);

    assert!(condition.evaluate(&context));

    let context2 = skill::SkillContext::new("I need to write");
    assert!(!condition.evaluate(&context2));
}

#[test]
fn test_activation_condition_user_request() {
    let context = skill::SkillContext::new("test").with_user_requested(true);
    assert!(ActivationCondition::UserRequest.evaluate(&context));

    let context2 = skill::SkillContext::new("test");
    assert!(!ActivationCondition::UserRequest.evaluate(&context2));
}

#[test]
fn test_activation_condition_and() {
    let context = skill::SkillContext::new("code here").with_user_requested(true);
    let condition = ActivationCondition::And(vec![
        ActivationCondition::KeywordTrigger(vec!["code".to_string()]),
        ActivationCondition::UserRequest,
    ]);

    assert!(condition.evaluate(&context));

    let context2 = skill::SkillContext::new("code here");
    assert!(!condition.evaluate(&context2));
}

#[test]
fn test_activation_condition_or() {
    let condition = ActivationCondition::Or(vec![
        ActivationCondition::KeywordTrigger(vec!["code".to_string()]),
        ActivationCondition::UserRequest,
    ]);

    let ctx1 = skill::SkillContext::new("code here");
    assert!(condition.evaluate(&ctx1));

    let ctx2 = skill::SkillContext::new("anything").with_user_requested(true);
    assert!(condition.evaluate(&ctx2));

    let ctx3 = skill::SkillContext::new("anything");
    assert!(!condition.evaluate(&ctx3));
}

#[test]
fn test_activation_condition_not() {
    let condition = ActivationCondition::Not(Box::new(ActivationCondition::Always));
    let context = skill::SkillContext::new("test");
    assert!(!condition.evaluate(&context));
}

#[test]
fn test_skill_can_activate() {
    let skill = Skill::new("s1", "Test")
        .with_activation(ActivationCondition::KeywordTrigger(vec!["test".to_string()]));

    let ctx = skill::SkillContext::new("this is a test");
    assert!(skill.can_activate(&ctx));

    let ctx2 = skill::SkillContext::new("nothing here");
    assert!(!skill.can_activate(&ctx2));
}

#[test]
fn test_skill_disabled_cannot_activate() {
    let mut skill = Skill::new("s1", "Test").with_activation(ActivationCondition::Always);
    skill.disable();

    let ctx = skill::SkillContext::new("test");
    assert!(!skill.can_activate(&ctx));
}

#[test]
fn test_skill_activate_deactivate() {
    let mut skill = Skill::new("s1", "Test").with_activation(ActivationCondition::Always);

    let ctx = skill::SkillContext::new("test");
    assert_eq!(skill.state, SkillState::Inactive);

    assert!(skill.activate(&ctx));
    assert_eq!(skill.state, SkillState::Active);

    skill.deactivate();
    assert_eq!(skill.state, SkillState::Inactive);
}

#[test]
fn test_skill_enable_disable() {
    let mut skill = Skill::new("s1", "Test").with_activation(ActivationCondition::Always);
    skill.disable();
    assert_eq!(skill.state, SkillState::Disabled);

    skill.enable();
    assert_eq!(skill.state, SkillState::Inactive);
}

#[test]
fn test_agent_persona_new() {
    let identity = AgentIdentity::new("id", "Agent");
    let persona = AgentPersona::new(identity);

    assert_eq!(persona.identity().name, "Agent");
    assert_eq!(persona.skill_count(), 0);
}

#[test]
fn test_agent_persona_add_and_get_skill() {
    let identity = AgentIdentity::new("id", "Agent");
    let persona = AgentPersona::new(identity);

    let skill = Skill::new("s1", "TestSkill").with_activation(ActivationCondition::Always);
    persona.add_skill(skill);

    assert_eq!(persona.skill_count(), 1);
    assert!(persona.get_skill("s1").is_some());
    assert_eq!(persona.get_skill("s1").unwrap().name, "TestSkill");
}

#[test]
fn test_agent_persona_remove_skill() {
    let identity = AgentIdentity::new("id", "Agent");
    let persona = AgentPersona::new(identity);

    persona.add_skill(Skill::new("s1", "Test"));
    assert_eq!(persona.skill_count(), 1);

    assert!(persona.remove_skill("s1"));
    assert_eq!(persona.skill_count(), 0);
    assert!(!persona.remove_skill("s1"));
}

#[test]
fn test_agent_persona_activate_deactivate_skill() {
    let identity = AgentIdentity::new("id", "Agent");
    let persona = AgentPersona::new(identity);

    persona.add_skill(Skill::new("s1", "Test").with_activation(ActivationCondition::Always));

    assert_eq!(persona.active_skill_count(), 0);

    let activated = persona.activate_skill("s1").unwrap();
    assert!(activated);
    assert_eq!(persona.active_skill_count(), 1);

    let deactivated = persona.deactivate_skill("s1").unwrap();
    assert!(deactivated);
    assert_eq!(persona.active_skill_count(), 0);
}

#[test]
fn test_agent_persona_update_skills_for_context() {
    let identity = AgentIdentity::new("id", "Agent");
    let persona = AgentPersona::new(identity);

    persona.add_skill(
        Skill::new("s1", "Coding")
            .with_activation(ActivationCondition::KeywordTrigger(vec![
                "code".to_string()
            ])),
    );
    persona.add_skill(
        Skill::new("s2", "Research")
            .with_activation(ActivationCondition::KeywordTrigger(vec![
                "research".to_string()
            ])),
    );

    let newly = persona.update_skills_for_context("write code", false);
    assert_eq!(newly.len(), 1);
    assert_eq!(persona.active_skill_count(), 1);

    let newly2 = persona.update_skills_for_context("do research and code", false);
    assert_eq!(newly2.len(), 1);
    assert_eq!(persona.active_skill_count(), 2);

    let _ = persona.update_skills_for_context("nothing", false);
    assert_eq!(persona.active_skill_count(), 0);
}

#[test]
fn test_agent_persona_build_prompt() {
    let identity = AgentIdentity::new("id", "Helper")
        .with_description("A helpful assistant")
        .with_tone("friendly");
    let persona = AgentPersona::new(identity);

    persona.add_skill(
        Skill::new("s1", "Coding")
            .with_activation(ActivationCondition::Always)
            .with_prompt_fragments(vec!["Write clean code.".to_string()]),
    );

    persona.activate_skill("s1").unwrap();

    let prompt = persona.build_prompt(DisclosureLevel::Basic);
    assert!(prompt.contains("Helper"));
    assert!(prompt.contains("Coding"));
    assert!(prompt.contains("Write clean code."));
}

#[test]
fn test_agent_persona_available_tools() {
    let identity = AgentIdentity::new("id", "Agent");
    let persona = AgentPersona::new(identity);

    persona.add_skill(
        Skill::new("s1", "Coding")
            .with_activation(ActivationCondition::Always)
            .with_tools(vec!["execute".to_string(), "read".to_string()]),
    );
    persona.add_skill(
        Skill::new("s2", "Search")
            .with_activation(ActivationCondition::Always)
            .with_tools(vec!["search".to_string(), "read".to_string()]),
    );

    persona.activate_skill("s1").unwrap();
    persona.activate_skill("s2").unwrap();

    let tools = persona.available_tools();
    assert_eq!(tools.len(), 3);
    assert!(tools.contains(&"execute".to_string()));
    assert!(tools.contains(&"read".to_string()));
    assert!(tools.contains(&"search".to_string()));
}

#[test]
fn test_agent_persona_set_disclosure_level() {
    let identity = AgentIdentity::new("id", "Agent");
    let persona = AgentPersona::new(identity);

    assert_eq!(persona.disclosure_level(), DisclosureLevel::Basic);

    persona.set_disclosure_level(DisclosureLevel::Full);
    assert_eq!(persona.disclosure_level(), DisclosureLevel::Full);
}

#[test]
fn test_agent_persona_list_skills() {
    let identity = AgentIdentity::new("id", "Agent");
    let persona = AgentPersona::new(identity);

    persona.add_skill(Skill::new("s1", "Low").with_priority(1));
    persona.add_skill(Skill::new("s2", "High").with_priority(100));
    persona.add_skill(Skill::new("s3", "Medium").with_priority(50));

    let skills = persona.list_skills();
    assert_eq!(skills.len(), 3);
    assert_eq!(skills[0].name, "High");
    assert_eq!(skills[1].name, "Medium");
    assert_eq!(skills[2].name, "Low");
}

#[test]
fn test_identity_with_capabilities() {
    let identity = AgentIdentity::new("id", "Dev")
        .with_capabilities(vec!["coding".to_string(), "debugging".to_string()]);

    assert_eq!(identity.capabilities.len(), 2);
    assert!(identity.capabilities.contains(&"coding".to_string()));
}

#[test]
fn test_identity_with_metadata() {
    let identity =
        AgentIdentity::new("id", "Test").with_metadata(json!({ "version": "1.0" }));

    assert_eq!(identity.metadata["version"], "1.0");
}

#[test]
fn test_skill_with_cooldown() {
    let mut skill = Skill::new("s1", "Test")
        .with_activation(ActivationCondition::Always)
        .with_cooldown(1000);

    let ctx = skill::SkillContext::new("test");
    assert!(skill.activate(&ctx));
    assert!(!skill.can_activate(&ctx));
}

#[test]
fn test_skill_with_prompt_fragments() {
    let skill = Skill::new("s1", "Test")
        .with_prompt_fragments(vec!["fragment1".to_string(), "fragment2".to_string()]);

    assert_eq!(skill.prompt_fragments.len(), 2);
}

#[test]
fn test_identity_with_system_prompt() {
    let identity = AgentIdentity::new("id", "Bot").with_system_prompt("You are helpful.");
    assert_eq!(identity.system_prompt, "You are helpful.");
}