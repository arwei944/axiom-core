//! Security layer for MCP protocol bridge.
//! Implements Permission → Rules → Axiom → Human-in-the-loop security model.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;

use crate::protocol::McpError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PermissionLevel {
    #[default]
    None,
    Read,
    Write,
    Admin,
}

#[derive(Debug, Clone)]
pub struct ToolPermission {
    pub tool_name: String,
    pub required_level: PermissionLevel,
    pub requires_approval: bool,
    pub approval_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub actor_id: String,
    pub permission_level: PermissionLevel,
    pub is_human: bool,
    pub request_metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub id: String,
    pub tool_name: String,
    pub arguments: Value,
    pub actor_id: String,
    pub required_level: PermissionLevel,
    pub timestamp: u64,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    TimedOut,
}

pub struct ApprovalManager {
    pending: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
    status: Arc<RwLock<HashMap<String, ApprovalStatus>>>,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn request_approval(&self, request: ApprovalRequest) {
        let id = request.id.clone();
        self.pending.write().insert(id.clone(), request);
        self.status.write().insert(id, ApprovalStatus::Pending);
    }

    pub fn approve(&self, request_id: &str) -> bool {
        if let Some(status) = self.status.write().get_mut(request_id) {
            if *status == ApprovalStatus::Pending {
                *status = ApprovalStatus::Approved;
                return true;
            }
        }
        false
    }

    pub fn reject(&self, request_id: &str) -> bool {
        if let Some(status) = self.status.write().get_mut(request_id) {
            if *status == ApprovalStatus::Pending {
                *status = ApprovalStatus::Rejected;
                return true;
            }
        }
        false
    }

    pub fn get_status(&self, request_id: &str) -> Option<ApprovalStatus> {
        self.status.read().get(request_id).cloned()
    }

    pub fn list_pending(&self) -> Vec<ApprovalRequest> {
        self.pending.read().values().cloned().collect()
    }
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SecurityManager {
    permissions: Arc<RwLock<HashMap<String, ToolPermission>>>,
    approval_manager: Arc<ApprovalManager>,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            permissions: Arc::new(RwLock::new(HashMap::new())),
            approval_manager: Arc::new(ApprovalManager::new()),
        }
    }

    pub fn register_permission(&self, permission: ToolPermission) {
        self.permissions
            .write()
            .insert(permission.tool_name.clone(), permission);
    }

    pub fn check_permission(
        &self,
        tool_name: &str,
        context: &SecurityContext,
    ) -> Result<bool, McpError> {
        let permissions = self.permissions.read();
        let permission = match permissions.get(tool_name) {
            Some(p) => p.clone(),
            None => return Ok(true),
        };

        if context.permission_level < permission.required_level {
            return Err(McpError::PermissionDenied(format!(
                "required level {:?}, got {:?}",
                permission.required_level, context.permission_level
            )));
        }

        Ok(true)
    }

    pub fn check_approval(
        &self,
        tool_name: &str,
        arguments: &Value,
        context: &SecurityContext,
    ) -> Result<bool, McpError> {
        let permissions = self.permissions.read();
        let permission = match permissions.get(tool_name) {
            Some(p) => p.clone(),
            None => return Ok(true),
        };

        if !permission.requires_approval {
            return Ok(true);
        }

        if context.is_human {
            return Ok(true);
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        let request = ApprovalRequest {
            id: request_id.clone(),
            tool_name: tool_name.to_string(),
            arguments: arguments.clone(),
            actor_id: context.actor_id.clone(),
            required_level: permission.required_level,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            timeout_ms: permission.approval_timeout_ms,
        };

        self.approval_manager.request_approval(request);

        Err(McpError::RequiresApproval(request_id))
    }

    pub fn approval_manager(&self) -> Arc<ApprovalManager> {
        self.approval_manager.clone()
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for PermissionLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PermissionLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_val = match self {
            PermissionLevel::None => 0,
            PermissionLevel::Read => 1,
            PermissionLevel::Write => 2,
            PermissionLevel::Admin => 3,
        };
        let other_val = match other {
            PermissionLevel::None => 0,
            PermissionLevel::Read => 1,
            PermissionLevel::Write => 2,
            PermissionLevel::Admin => 3,
        };
        self_val.cmp(&other_val)
    }
}
