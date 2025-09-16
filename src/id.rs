use std::fmt::{Display, Formatter, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Serialize, Deserialize)]
pub enum Id {
    None,
    Guid(Uuid),
    Loc(String),
    CsType(String),
}

impl Default for Id {
    fn default() -> Self {
        Id::Guid(Uuid::nil())
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Guid(uuid) => write!(f, "guid:{}", uuid),
            Self::Loc(name) => write!(f, "loc:{}", name),
            Self::CsType(name) => write!(f, "cs_type:{}", name),
        }
    }
}
