use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, PartialEq, Eq, Hash, Debug, Clone, Serialize)]
pub struct Id(Uuid);

impl Id {
    pub fn new_uuid(id: Uuid) -> Self {
        Id(id)
    }

    pub fn new_loc(name: &str) -> Self {
        Id(Uuid::new_v5(&Uuid::NAMESPACE_URL, format!("loc:{name}").as_bytes()))
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
