use std::fmt::{Display, Formatter, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Serialize, Deserialize, Default)]
pub enum Id {
    #[default]
    None,
    Guid(Uuid),
    Loc(String),
    CsType { name: String, namespace: Option<String> },
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::None => write!(f, "<no id>"),
            Self::Guid(uuid) => write!(f, "guid:{}", uuid),
            Self::Loc(name) => write!(f, "loc:{}", name),
            Self::CsType { name, namespace: Some(ns) } => write!(f, "cs_type:{ns}.{name}"),
            Self::CsType { name, namespace: None } => write!(f, "cs_type:{}", name),
        }
    }
}
