use agent_sdk_core::{PolicyKind, PolicyRef};

pub fn discovery_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}
