use axiom_kernel::plugin::composer::{Composer, SystemComposition};

#[test]
fn test_composer_from_str() {
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
    let composition = Composer::from_str(toml).unwrap();
    assert_eq!(composition.system.name, "test-system");
    assert_eq!(composition.plugins.len(), 1);
    assert_eq!(composition.plugins[0].id, "echo");
    assert_eq!(composition.connections.len(), 1);
}
