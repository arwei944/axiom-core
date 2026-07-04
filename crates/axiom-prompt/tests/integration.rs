use axiom_prompt::*;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_template_render_simple() {
    let template = PromptTemplate::new("greeting", "Hello, {{name}}! Welcome to {{place}}.")
        .with_variable(TemplateVariable::new("name", VariableType::String))
        .with_variable(TemplateVariable::new("place", VariableType::String));

    let mut values = HashMap::new();
    values.insert("name".to_string(), json!("Alice"));
    values.insert("place".to_string(), json!("Wonderland"));

    let result = template.render(&values).unwrap();
    assert_eq!(result, "Hello, Alice! Welcome to Wonderland.");
}

#[test]
fn test_template_missing_variable() {
    let template = PromptTemplate::new("test", "Hello, {{name}}!")
        .with_variable(TemplateVariable::new("name", VariableType::String));

    let values = HashMap::new();
    let result = template.render(&values);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PromptError::MissingVariable(_)
    ));
}

#[test]
fn test_template_default_value() {
    let template = PromptTemplate::new("greeting", "Hello, {{name}}!").with_variable(
        TemplateVariable::new("name", VariableType::String).with_default(json!("World")),
    );

    let values = HashMap::new();
    let result = template.render(&values).unwrap();
    assert_eq!(result, "Hello, World!");
}

#[test]
fn test_template_type_validation() {
    let template = PromptTemplate::new("test", "Count: {{count}}")
        .with_variable(TemplateVariable::new("count", VariableType::Number));

    let mut values = HashMap::new();
    values.insert("count".to_string(), json!("not a number"));

    let result = template.render(&values);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PromptError::InvalidType(_)));
}

#[test]
fn test_template_number_variable() {
    let template = PromptTemplate::new("math", "Result: {{value}}")
        .with_variable(TemplateVariable::new("value", VariableType::Number));

    let mut values = HashMap::new();
    values.insert("value".to_string(), json!(42));

    let result = template.render(&values).unwrap();
    assert_eq!(result, "Result: 42");
}

#[test]
fn test_template_boolean_variable() {
    let template = PromptTemplate::new("flag", "Enabled: {{enabled}}")
        .with_variable(TemplateVariable::new("enabled", VariableType::Boolean));

    let mut values = HashMap::new();
    values.insert("enabled".to_string(), json!(true));

    let result = template.render(&values).unwrap();
    assert_eq!(result, "Enabled: true");
}

#[test]
fn test_template_list_variable() {
    let template = PromptTemplate::new("items", "Items: {{items}}")
        .with_variable(TemplateVariable::new("items", VariableType::List));

    let mut values = HashMap::new();
    values.insert("items".to_string(), json!(["a", "b", "c"]));

    let result = template.render(&values).unwrap();
    assert_eq!(result, "Items: a, b, c");
}

#[test]
fn test_variable_names() {
    let template = PromptTemplate::new("test", "{{a}} {{b}} {{c}}")
        .with_variable(TemplateVariable::new("a", VariableType::String))
        .with_variable(TemplateVariable::new("b", VariableType::String))
        .with_variable(TemplateVariable::new("c", VariableType::String).optional());

    let names = template.variable_names();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
    assert!(names.contains(&"c"));
}

#[test]
fn test_required_and_optional_variables() {
    let template = PromptTemplate::new("test", "{{a}} {{b}}")
        .with_variable(TemplateVariable::new("a", VariableType::String))
        .with_variable(TemplateVariable::new("b", VariableType::String).optional());

    assert_eq!(template.required_variables().len(), 1);
    assert_eq!(template.optional_variables().len(), 1);
}

#[test]
fn test_template_composition() {
    let system = PromptTemplate::new(
        "system",
        "You are a helpful assistant.\n{%body%}\nEnd instructions.",
    )
    .with_section("body");

    let body = PromptTemplate::new("body", "Task: {{task}}")
        .with_variable(TemplateVariable::new("task", VariableType::String));

    let composed = system.compose(&body, "body").unwrap();

    let mut values = HashMap::new();
    values.insert("task".to_string(), json!("write code"));

    let result = composed.render(&values).unwrap();
    assert_eq!(
        result,
        "You are a helpful assistant.\nTask: write code\nEnd instructions."
    );
}

#[test]
fn test_template_composition_missing_section() {
    let system = PromptTemplate::new("system", "No section here.");
    let body = PromptTemplate::new("body", "Body content");

    let result = system.compose(&body, "body");
    assert!(result.is_err());
}

#[test]
fn test_registry_register_and_get() {
    let mut registry = registry::TemplateRegistry::new();

    let template = PromptTemplate::new("greeting", "Hello, {{name}}!")
        .with_variable(TemplateVariable::new("name", VariableType::String));

    registry.register(template).unwrap();

    assert!(registry.has_template("greeting"));
    assert!(registry.get_latest("greeting").is_some());
}

#[test]
fn test_registry_version_management() {
    let mut registry = registry::TemplateRegistry::new();

    let v1 = PromptTemplate::new("greeting", "Hello, {{name}}!")
        .with_version("1.0.0")
        .with_variable(TemplateVariable::new("name", VariableType::String));

    let v2 = PromptTemplate::new("greeting", "Hi, {{name}}!")
        .with_version("2.0.0")
        .with_variable(TemplateVariable::new("name", VariableType::String));

    registry.register(v1).unwrap();
    registry.register(v2).unwrap();

    assert_eq!(registry.list_versions("greeting").len(), 2);

    let latest = registry.get_latest("greeting").unwrap();
    assert_eq!(latest.version, "2.0.0");

    let v1_template = registry.get_version("greeting", "1.0.0").unwrap();
    assert_eq!(v1_template.version, "1.0.0");
}

#[test]
fn test_registry_version_conflict() {
    let mut registry = registry::TemplateRegistry::new();

    let t1 = PromptTemplate::new("test", "Template v1").with_version("1.0.0");
    let t2 = PromptTemplate::new("test", "Template v1 again").with_version("1.0.0");

    registry.register(t1).unwrap();
    let result = registry.register(t2);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PromptError::VersionConflict(_)
    ));
}

#[test]
fn test_registry_render() {
    let mut registry = registry::TemplateRegistry::new();

    let template = PromptTemplate::new("greeting", "Hello, {{name}}!")
        .with_variable(TemplateVariable::new("name", VariableType::String));

    registry.register(template).unwrap();

    let mut values = HashMap::new();
    values.insert("name".to_string(), json!("Bob"));

    let result = registry.render_latest("greeting", &values).unwrap();
    assert_eq!(result, "Hello, Bob!");
}

#[test]
fn test_registry_not_found() {
    let registry = registry::TemplateRegistry::new();
    let values = HashMap::new();
    let result = registry.render_latest("nonexistent", &values);

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PromptError::NotFound(_)));
}

#[test]
fn test_template_with_description() {
    let template = PromptTemplate::new("test", "content").with_description("A test template");

    assert_eq!(template.description.as_deref(), Some("A test template"));
}

#[test]
fn test_variable_with_description() {
    let var =
        TemplateVariable::new("name", VariableType::String).with_description("The user's name");

    assert_eq!(var.description.as_deref(), Some("The user's name"));
}

#[test]
fn test_registry_counts() {
    let mut registry = registry::TemplateRegistry::new();

    assert_eq!(registry.template_count(), 0);
    assert_eq!(registry.total_versions(), 0);

    registry
        .register(PromptTemplate::new("a", "A").with_version("1.0.0"))
        .unwrap();
    registry
        .register(PromptTemplate::new("a", "A2").with_version("2.0.0"))
        .unwrap();
    registry
        .register(PromptTemplate::new("b", "B").with_version("1.0.0"))
        .unwrap();

    assert_eq!(registry.template_count(), 2);
    assert_eq!(registry.total_versions(), 3);
}

#[test]
fn test_registry_remove() {
    let mut registry = registry::TemplateRegistry::new();

    registry
        .register(PromptTemplate::new("test", "Test").with_version("1.0.0"))
        .unwrap();

    assert!(registry.has_template("test"));
    assert!(registry.remove("test"));
    assert!(!registry.has_template("test"));
    assert!(!registry.remove("test"));
}

#[test]
fn test_registry_remove_version() {
    let mut registry = registry::TemplateRegistry::new();

    registry
        .register(PromptTemplate::new("test", "v1").with_version("1.0.0"))
        .unwrap();
    registry
        .register(PromptTemplate::new("test", "v2").with_version("2.0.0"))
        .unwrap();

    assert!(registry.has_version("test", "1.0.0"));
    assert!(registry.remove_version("test", "1.0.0"));
    assert!(!registry.has_version("test", "1.0.0"));
    assert!(registry.has_version("test", "2.0.0"));
}

#[test]
fn test_variable_type_strings() {
    assert_eq!(VariableType::String.as_str(), "string");
    assert_eq!(VariableType::Number.as_str(), "number");
    assert_eq!(VariableType::Boolean.as_str(), "boolean");
    assert_eq!(VariableType::List.as_str(), "list");
    assert_eq!(VariableType::Object.as_str(), "object");
}

#[test]
fn test_render_with_defaults() {
    let template = PromptTemplate::new("greeting", "Hello, {{name}}!").with_variable(
        TemplateVariable::new("name", VariableType::String).with_default(json!("World")),
    );

    let result = template.render_with_defaults().unwrap();
    assert_eq!(result, "Hello, World!");
}
