use std::{
    collections::HashSet,
    path::PathBuf,
};
use serde::{Deserialize, Serialize};
use crate::{
    id::Id,
    database::Database,
    parsers::{unity::UnityObject, ParseError, Parser},
};

#[derive(Deserialize, Serialize)]
pub enum AssetType {
    Prefab,
    Scene,
    Texture,
    Model,
    Audio,
    Script,
    Unknown,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetType::Prefab => write!(f, "Prefab"),
            AssetType::Scene => write!(f, "Scene"),
            AssetType::Texture => write!(f, "Texture"),
            AssetType::Model => write!(f, "Model"),
            AssetType::Audio => write!(f, "Audio"),
            AssetType::Script => write!(f, "Script"),
            AssetType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Asset {
    pub id: Id,
    pub asset_type: AssetType,
    pub path: PathBuf,
    pub loc_roots: HashSet<PathBuf>,
    pub dependencies: HashSet<Id>,

    #[serde(skip)]
    pub dependents: HashSet<Id>,
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
            loc_roots: HashSet::new(),
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        }
    }

    pub fn read_contents(&mut self, relative_to: Option<&PathBuf>) -> Result<(), ParseError> {
        match self.asset_type {
            AssetType::Prefab => UnityObject::parse(self, relative_to),
            _ => { Ok(()) },
        }
    }

    pub fn bind<'a, 'b>(&'a self, db: &'b Database) -> BoundAsset<'a, 'b> {
        BoundAsset {
            asset: self,
            db,
        }
    }
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Asset ID: {}", self.id)?;
        writeln!(f, "Type: {}", self.asset_type)?;
        writeln!(f, "Path: {}", self.path.display())?;
        writeln!(f, "Dependents ({}):", self.dependents.len())?;
        for dep in &self.dependents {
            writeln!(f, " - {}", dep)?;
        }
        writeln!(f, "Dependencies ({}):", self.dependencies.len())?;
        for dep in &self.dependencies {
            writeln!(f, " - {}", dep)?;
        }
        Ok(())
    }
}

pub struct BoundAsset<'a, 'b> {
    pub asset: &'a Asset,
    pub db: &'b Database,
}

impl<'a, 'b> std::fmt::Display for BoundAsset<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Asset ID: {}", self.asset.id)?;
        writeln!(f, "Type: {}", self.asset.asset_type)?;
        writeln!(f, "Path: {}", self.asset.path.display())?;

        writeln!(f, "Dependents ({}):", self.asset.dependents.len())?;
        let mut deps: Vec<String> = self.asset.dependents.iter()
            .map(|id| 
                self.db.asset(id)
                .and_then(|a|
                    Some(a.path.display().to_string())
                )
                .unwrap_or(id.to_string())
            ).collect();
        deps.sort();
        for dep in &deps {
            writeln!(f, " - {}", dep)?;
        }

        writeln!(f, "Dependencies ({}):", self.asset.dependencies.len())?;
        let mut deps: Vec<String> = self.asset.dependencies.iter()
            .map(|id| 
                self.db.asset(id)
                .and_then(|a| Some(a.path.display().to_string()))
                .unwrap_or(id.to_string())
            ).collect();
        deps.sort();

        for dep in deps {
            writeln!(f, " - {}", dep)?;
        }
        Ok(())
    }
}