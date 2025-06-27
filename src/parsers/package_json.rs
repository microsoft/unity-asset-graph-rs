use std::collections::HashMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PackageJson {
    pub dependencies: Option<HashMap<String, String>>,
}