pub mod manifest_json;
pub mod package_json;
mod unity;
mod csharp;
mod directory;

#[cfg(feature = "locstring")]
mod loc_text;
#[cfg(feature = "locstring")]
mod loc_resource;
#[cfg(feature = "locstring")]
mod loc_override;

pub use csharp::{type_broker::TypeBroker, qualified_name::QualifiedName};

use std::{
    error::Error,
    sync::{Arc, Mutex},
    path::{Path, PathBuf},
};
use crate::{
    asset::Asset,
    asset_type::AssetType,
};

#[derive(Debug, Default)]
pub struct ParseError {
    pub path: PathBuf,
    pub message: String,
    pub inner: Option<Box<dyn Error + Send>>,
}

impl ParseError {
    pub fn new(path: &Path, message: String) -> Self {
        Self {
            path: path.to_path_buf(),
            message,
            inner: None,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(e) = self.inner.as_ref() {
            write!(f, "{}: {} ({})", self.path.display(), self.message, e)
        } else {
            write!(f, "{}: {}", self.path.display(), self.message)
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse(asset: &mut Asset, relative_to: Option<&PathBuf>, broker: &Arc<Mutex<TypeBroker>>) -> Result<Vec<Asset>, ParseError> {
    if asset.path.is_none() {
        return Ok(vec![]);
    }

    // populate directory dependency for all assets
    directory::parse(asset, relative_to)?;

    if asset.asset_type.is_unity() {
        return unity::parse(asset, relative_to);
    }
    if let AssetType::CsFile = asset.asset_type {
        return csharp::parse(asset, relative_to, broker);
    }
    #[cfg(feature = "locstring")]
    if let Some(filename) = asset.path.as_ref().unwrap().file_name().and_then(|f| f.to_str())
        && filename.ends_with("Resource.en-us.json") {
        asset.asset_type = AssetType::LocResource;
        return loc_resource::parse(asset, relative_to);
    }
    
    Ok(vec![])
}
