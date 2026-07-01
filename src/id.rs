use crate::parser::QualifiedNameOwned;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};
use uuid::Uuid;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Serialize, Deserialize, Default)]
pub enum Id {
    #[default]
    None,
    Guid(Uuid),
    Loc(String),
    /// A C# type identified by its full name parts in reverse order (e.g. ["MyClass", "MyNamespace"])
    CsType(QualifiedNameOwned),
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::None => write!(f, "<no id>"),
            Self::Guid(uuid) => write!(f, "guid:{}", uuid),
            Self::Loc(name) => write!(f, "loc:{}", name),
            Self::CsType(name) => write!(f, "cs_type:{name}"),
        }
    }
}
