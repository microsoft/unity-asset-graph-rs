use std::{
    path::PathBuf,
    collections::HashSet,
};
use crate::id::Id;

pub enum AssetType {
    Prefab,
    Scene,
    Texture,
    Model,
    Audio,
    Script,
    Unknown,
}

pub struct Asset {
    pub id: Id,
    pub asset_type: AssetType,
    pub path: PathBuf,
    pub dependencies: HashSet<Id>,
}

impl Asset {
    pub fn new(id: Id, path: PathBuf) -> Self {
        let asset_type = match path.extension().and_then(|s| s.to_str()) {
            Some("prefab") => AssetType::Prefab,
            Some("unity") | Some("scene") => AssetType::Scene,
            Some("png") | Some("jpg") | Some("jpeg") => AssetType::Texture,
            Some("fbx") | Some("obj") => AssetType::Model,
            Some("wav") | Some("mp3") => AssetType::Audio,
            Some("cs") | Some("js") => AssetType::Script,
            _ => AssetType::Unknown,
        };
        
        Self {
            id,
            asset_type,
            path,
            dependencies: HashSet::new(),
        }
    }
}