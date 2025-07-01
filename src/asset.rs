use std::{
    path::PathBuf,
    collections::HashSet,
};
use serde::Serialize;
use crate::{
    id::Id,
    parsers::unity,
};

#[derive(Serialize)]
pub enum AssetType {
    Prefab,
    Scene,
    Texture,
    Model,
    Audio,
    Script,
    Unknown,
}

#[derive(Serialize)]
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

        let deps = match asset_type {
            AssetType::Prefab | AssetType::Scene => {
                match unity::parse(&path) {
                    Ok(deps) => deps,
                    Err(e) => {
                        eprintln!("Failed to parse asset at {} as Unity prefab or scene: {}", path.display(), e);
                        HashSet::new()
                    }
                }
            },
            _ => HashSet::new(),
        };
        
        Self {
            id,
            asset_type,
            path,
            dependencies: deps,
        }
    }
}