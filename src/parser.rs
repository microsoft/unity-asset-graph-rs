mod csharp;
mod directory;
pub mod manifest_json;
pub mod package_json;
mod unity;

#[cfg(feature = "locstring")]
mod loc_override;
#[cfg(feature = "locstring")]
mod loc_resource;
#[cfg(feature = "locstring")]
mod loc_text;

pub use csharp::{
    qualified_name::{QualifiedName, QualifiedNameOwned},
    type_broker::TypeBroker,
};

use crate::{asset::Asset, asset_type::AssetType};
use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
pub struct ParseError {
    pub path: PathBuf,
    pub message: String,
    pub inner: Option<Box<dyn Error + Send>>,
}

impl ParseError {
    pub fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
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

pub fn parse(
    asset: &mut Asset,
    relative_to: &Path,
    broker: &Arc<Mutex<TypeBroker>>,
) -> Result<Vec<Asset>, ParseError> {
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
        && filename.ends_with("Resource.en-us.json")
    {
        asset.asset_type = AssetType::LocResource;
        return loc_resource::parse(asset, relative_to);
    }

    Ok(vec![])
}
