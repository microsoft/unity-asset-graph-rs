use uuid::Uuid;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum Id {
    Guid(Uuid),
    Loc(String),
}