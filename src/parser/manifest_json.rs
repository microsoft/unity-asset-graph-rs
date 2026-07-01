use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct ManifestJson {
    pub dependencies: HashMap<String, String>,
}
