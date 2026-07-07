use axiom_kernel::plugin::composer::Composer;

#[test]
fn test_composer_parse() {
    let toml = r#"
[system]
name = "test-system"

[[plugins]]
id = "echo"
kind = "Tool"
config = {}
instance = 1

[[connections]]
from = ["echo", "out"]
to = ["counter", "in"]
"#;
    let composition = Composer::parse(toml).unwrap();
    assert_eq!(composition.system.name, "test-system");
    assert_eq!(composition.plugins.len(), 1);
    assert_eq!(composition.plugins[0].id, "echo");
    assert_eq!(composition.connections.len(), 1);
}
