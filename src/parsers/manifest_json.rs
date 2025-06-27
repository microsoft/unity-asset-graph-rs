use std::collections::HashMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ManifestJson {
    pub dependencies: HashMap<String, String>,
}