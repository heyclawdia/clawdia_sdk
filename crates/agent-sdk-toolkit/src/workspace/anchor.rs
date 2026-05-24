use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HashLineAnchor {
    pub line: usize,
    pub before_hash: String,
}
