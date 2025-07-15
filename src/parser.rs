use std::path::PathBuf;
use crate::asset::Asset;

pub mod manifest_json;
pub mod package_json;
mod unity;
mod loc_text;
mod loc_manager;
mod csharp;

#[derive(Debug)]
pub struct ParseError {
    message: String,
    inner: Option<Box<dyn std::error::Error>>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(inner) = &self.inner {
            write!(f, "{}: {}", self.message, inner)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse(asset: &mut Asset, relative_to: Option<&PathBuf>) -> Result<Vec<Asset>, ParseError> {
    match asset.path.extension().and_then(|s| s.to_str()) {
        Some("prefab") | Some("unity") | Some("scene") | Some("asset") => unity::parse_unity(asset, relative_to),
        Some("cs") => csharp::parse_csharp(asset, relative_to),
        _ => Ok(vec![]), // Not a known file type
    }
}
