use std::{
    collections::HashSet,
    path::PathBuf,
};
use serde::{Deserialize, Serialize};
use crate::{
    id::Id,
    database::Database,
};

#[derive(Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum AssetType {
    #[default]
    Unknown,
    Directory,
    Prefab,
    Scene,
    Texture,
    Model,
    Audio,
    Script,
    LocResource,
    LocString,
    BrokenRef,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetType::Unknown => write!(f, "Unknown"),
            AssetType::Directory => write!(f, "Directory"),
            AssetType::Prefab => write!(f, "Prefab"),
            AssetType::Scene => write!(f, "Scene"),
            AssetType::Texture => write!(f, "Texture"),
            AssetType::Model => write!(f, "Model"),
            AssetType::Audio => write!(f, "Audio"),
            AssetType::Script => write!(f, "Script"),
            AssetType::LocResource => write!(f, "Localization Resource"),
            AssetType::LocString => write!(f, "Localized String"),
            AssetType::BrokenRef => write!(f, "Broken Reference"),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct Asset {
    pub id: Id,
    pub asset_type: AssetType,
    pub path: Option<PathBuf>,
    pub dependencies: HashSet<Id>,

    #[serde(skip)]
    pub dependents: HashSet<Id>,
}

impl Asset {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
    pub fn new_with_path(id: Id, path: PathBuf) -> Self {
        let asset_type = if path.is_dir() {
            AssetType::Directory
        }
        else {
            match &path.extension().and_then(|s| s.to_str()) {
                Some("prefab") => AssetType::Prefab,
                Some("unity") | Some("scene") => AssetType::Scene,
                Some("png") | Some("jpg") | Some("jpeg") => AssetType::Texture,
                Some("fbx") | Some("obj") => AssetType::Model,
                Some("wav") | Some("mp3") => AssetType::Audio,
                Some("cs") | Some("js") => AssetType::Script,
                _ => AssetType::Unknown,
            }
        };

        Self {
            id,
            asset_type,
            path: Some(path),
            ..Default::default()
        }
    }

    pub fn bind<'a, 'b>(&'a self, db: &'b Database) -> BoundAsset<'a, 'b> {
        BoundAsset {
            asset: self,
            db,
            indent: 0,
        }
    }
}

pub struct BoundAsset<'a, 'b> {
    pub asset: &'a Asset,
    pub db: &'b Database,
    indent: usize,
}

impl<'a, 'b> BoundAsset<'a, 'b> {
    pub fn indent(self) -> Self {
        Self {
            asset: self.asset,
            db: self.db,
            indent: self.indent + 1,
        }
    }

    pub fn unindent(self) -> Self{
        Self {
            asset: self.asset,
            db: self.db,
            indent: self.indent.saturating_sub(1),
        }
    }
}

impl<'a, 'b> std::fmt::Display for BoundAsset<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let first_indent = format!("{}- ", "  ".repeat(self.indent));
        let indent_str = "  ".repeat(self.indent + 1);
        writeln!(f, "{first_indent}Asset ID: {}", self.asset.id)?;
        writeln!(f, "{indent_str}Type: {}", self.asset.asset_type)?;
        if let Some(path) = &self.asset.path {
            writeln!(f, "{indent_str}Path: {}", path.display())?;
        }

        let mut deps = vec![];
        for dep_id in self.asset.dependents.iter() {
            if let Some(dep_asset) = self.db.asset(dep_id) {
                if let Some(path) = &dep_asset.path {
                    deps.push(path.display().to_string());
                }
                else {
                    deps.push(dep_asset.id.to_string());
                }
            }
            else {
                deps.push(dep_id.to_string());
            }
        }
        deps.sort();

        writeln!(f, "{indent_str}Dependents ({}):", deps.len())?;
        for dep in &deps {
            writeln!(f, "{indent_str}- {}", dep)?;
        }

        let mut deps = vec![];
        for dep_id in self.asset.dependencies.iter() {
            if let Some(dep_asset) = self.db.asset(dep_id) {
                if let Some(path) = &dep_asset.path {
                    deps.push(path.display().to_string());
                }
                else {
                    deps.push(dep_asset.id.to_string());
                }
            }
            else {
                deps.push(dep_id.to_string());
            }
        }
        deps.sort();

        writeln!(f, "{indent_str}Dependencies ({}):", deps.len())?;
        for dep in deps {
            writeln!(f, "{indent_str}- {}", dep)?;
        }
        Ok(())
    }
}