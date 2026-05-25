use agent_sdk_core::{PolicyKind, PolicyRef};

pub fn memory_read_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Permission, id)
}
