use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub enum Id {
    Guid(Uuid),
    Loc(String),
}

impl Default for Id {
    fn default() -> Self {
        Id::Guid(Uuid::nil())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Id::Guid(uuid) => write!(f, "guid:{}", uuid),
            Id::Loc(name) => write!(f, "loc:{}", name),
        }
    }
}
