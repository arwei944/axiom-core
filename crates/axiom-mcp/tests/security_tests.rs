use axiom_mcp::protocol::McpError;
use axiom_mcp::security::{PermissionLevel, SecurityContext, SecurityManager, ToolPermission};

#[test]
fn test_unregistered_tool_denied() {
    let manager = SecurityManager::new();

    let context = SecurityContext {
        actor_id: "test_actor".to_string(),
        permission_level: PermissionLevel::Admin,
        is_human: false,
        request_metadata: Default::default(),
    };

    let result = manager.check_permission("unregistered_tool", &context);

    assert!(result.is_err());
    if let Err(McpError::PermissionDenied(msg)) = result {
        assert!(msg.contains("unregistered_tool"));
        assert!(msg.contains("not registered"));
    } else {
        panic!("expected PermissionDenied error");
    }
}

#[test]
fn test_registered_tool_allowed() {
    let manager = SecurityManager::new();

    manager.register_permission(ToolPermission {
        tool_name: "registered_tool".to_string(),
        required_level: PermissionLevel::Read,
        requires_approval: false,
        approval_timeout_ms: None,
    });

    let context = SecurityContext {
        actor_id: "test_actor".to_string(),
        permission_level: PermissionLevel::Read,
        is_human: false,
        request_metadata: Default::default(),
    };

    let result = manager.check_permission("registered_tool", &context);
    assert!(result.is_ok());
}

#[test]
fn test_insufficient_permission_denied() {
    let manager = SecurityManager::new();

    manager.register_permission(ToolPermission {
        tool_name: "admin_tool".to_string(),
        required_level: PermissionLevel::Admin,
        requires_approval: false,
        approval_timeout_ms: None,
    });

    let context = SecurityContext {
        actor_id: "test_actor".to_string(),
        permission_level: PermissionLevel::Read,
        is_human: false,
        request_metadata: Default::default(),
    };

    let result = manager.check_permission("admin_tool", &context);

    assert!(result.is_err());
    if let Err(McpError::PermissionDenied(msg)) = result {
        assert!(msg.contains("required level"));
    } else {
        panic!("expected PermissionDenied error");
    }
}

#[test]
fn test_unregistered_tool_approval_denied() {
    let manager = SecurityManager::new();

    let context = SecurityContext {
        actor_id: "test_actor".to_string(),
        permission_level: PermissionLevel::Admin,
        is_human: false,
        request_metadata: Default::default(),
    };

    let result = manager.check_approval("unregistered_tool", &serde_json::json!({}), &context);

    assert!(result.is_err());
    if let Err(McpError::PermissionDenied(msg)) = result {
        assert!(msg.contains("unregistered_tool"));
    } else {
        panic!("expected PermissionDenied error");
    }
}
