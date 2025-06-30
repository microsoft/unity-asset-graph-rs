use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Meta {
    #[serde()]
    pub guid: Uuid,
}