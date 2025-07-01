use uuid::Uuid;
use serde::Serialize;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum Id {
    Guid(Uuid),
    Loc(String),
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Id::Guid(uuid) => serializer.serialize_str(format!("id:{uuid}").as_str()),
            Id::Loc(key) => serializer.serialize_str(format!("loc:{key}").as_str()),
        }
    }
}