use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct PackageJson {
    pub dependencies: Option<HashMap<String, String>>,
}
