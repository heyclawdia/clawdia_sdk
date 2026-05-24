use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    Public,
    Internal,
    ContentRefsOnly,
    Sensitive,
    Secret,
}

impl PrivacyClass {
    pub fn allows_raw_content_by_default(self) -> bool {
        matches!(self, Self::Public)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionClass {
    Ephemeral,
    RunScoped,
    SessionScoped,
    Durable,
    Persistent,
    HostPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustClass {
    Trusted,
    SdkGenerated,
    HostProvided,
    UserProvided,
    External,
    Untrusted,
}
