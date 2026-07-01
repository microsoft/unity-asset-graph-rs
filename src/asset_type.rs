use serde::{Deserialize, Serialize};
use std::{
    convert::From,
    fmt::{Display, Formatter, Result},
    path::PathBuf,
};

#[derive(Deserialize, Serialize, PartialEq, Eq, Default, Debug, Clone, Copy, Hash)]
pub enum AssetType {
    #[default]
    Unknown,
    Directory,
    Prefab,
    Scene,
    Asset,
    Texture,
    Model,
    Audio,
    CsFile,
    LocResource,
    LocString,
    BrokenRef,
    CsType,
}

impl AssetType {
    pub fn is_unity(&self) -> bool {
        match self {
            Self::Prefab | Self::Scene | Self::Asset => true,
            _ => false,
        }
    }
}

impl From<&PathBuf> for AssetType {
    fn from(value: &PathBuf) -> Self {
        match value.extension().and_then(|s| s.to_str()) {
            Some("prefab") => AssetType::Prefab,
            Some("unity") | Some("scene") => AssetType::Scene,
            Some("asset") => AssetType::Asset,
            Some("png") | Some("jpg") | Some("jpeg") => AssetType::Texture,
            Some("fbx") | Some("obj") => AssetType::Model,
            Some("wav") | Some("mp3") => AssetType::Audio,
            Some("cs") => AssetType::CsFile,
            _ => AssetType::Unknown,
        }
    }
}

impl Display for AssetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Directory => write!(f, "Directory"),
            Self::Prefab => write!(f, "Prefab"),
            Self::Scene => write!(f, "Scene"),
            Self::Texture => write!(f, "Texture"),
            Self::Model => write!(f, "Model"),
            Self::Audio => write!(f, "Audio"),
            Self::CsFile => write!(f, "C# Script"),
            Self::LocResource => write!(f, "Localization Resource"),
            Self::LocString => write!(f, "Localized String"),
            Self::BrokenRef => write!(f, "Broken Reference"),
            Self::Asset => write!(f, "Asset"),
            Self::CsType => write!(f, "C# Type"),
        }
    }
}
