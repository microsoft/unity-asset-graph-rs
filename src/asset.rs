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
    Prefab,
    Scene,
    Texture,
    Model,
    Audio,
    Script,
    BrokenRef,
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
            AssetType::BrokenRef => write!(f, "Broken Reference"),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct Asset {
    pub id: Id,
    pub asset_type: AssetType,
    pub path: PathBuf,
    pub loc_roots: Vec<String>,
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
        let mut a = Self::new(id);
        a.asset_type = match path.extension().and_then(|s| s.to_str()) {
            Some("prefab") => AssetType::Prefab,
            Some("unity") | Some("scene") => AssetType::Scene,
            Some("png") | Some("jpg") | Some("jpeg") => AssetType::Texture,
            Some("fbx") | Some("obj") => AssetType::Model,
            Some("wav") | Some("mp3") => AssetType::Audio,
            Some("cs") | Some("js") => AssetType::Script,
            _ => AssetType::Unknown,
        };
        a
    }

    pub fn bind<'a, 'b>(&'a self, db: &'b Database) -> BoundAsset<'a, 'b> {
        BoundAsset {
            asset: self,
            db,
            indent: 0,
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
        let first_indent = format!("- {}", "  ".repeat(self.indent.saturating_sub(1)));
        let indent_str = "  ".repeat(self.indent);
        writeln!(f, "{first_indent}Asset ID: {}", self.asset.id)?;
        writeln!(f, "{indent_str}Type: {}", self.asset.asset_type)?;
        writeln!(f, "{indent_str}Path: {}", self.asset.path.display())?;

        let mut deps = vec![];
        for dep_id in self.asset.dependents.iter() {
            if let Some(dep_asset) = self.db.asset(dep_id) {
                deps.push(dep_asset.path.display().to_string());
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
                deps.push(dep_asset.path.display().to_string());
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