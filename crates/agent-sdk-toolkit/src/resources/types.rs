use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceReaderRequest {
    pub uri: String,
    pub max_bytes: u64,
}
