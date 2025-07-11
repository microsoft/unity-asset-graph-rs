use std::{
    collections::{HashSet, HashMap },
    path::PathBuf,
    fs,
};
use serde::{Deserialize, Serialize};
use crate::{asset::Asset, id::Id};

mod roots;
mod assets;

#[derive(Debug)]
pub struct DatabaseError {
    message: String,
    inner: Option<Box<dyn std::error::Error>>,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(inner) = &self.inner {
            write!(f, "{}: {}", self.message, inner)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for DatabaseError {}

#[derive(Deserialize, Serialize)]
pub struct Database {
    relative_to: Option<PathBuf>,
    roots: HashSet<PathBuf>,
    loc_roots: HashSet<PathBuf>,
    assets: HashMap<Id, Asset>,
}

impl Database {
    pub fn new(root: &str, relative_to: Option<&str>) -> Result<Self, DatabaseError> {
        let relative_to = if let Some(pathstr) = relative_to
            && let Ok(path) = fs::canonicalize(pathstr)
        {
            Some(path)
        }
        else {
            eprintln!("failed to canonicalize relative path '{relative_to:?}'");
            None
        };

        let mut db = Self {
            relative_to,
            roots: HashSet::new(),
            loc_roots: HashSet::new(),
            assets: HashMap::new(),
        };

        match db.add_root_str(root) {
            Ok(_) => Ok(db),
            Err(e) => Err(e),
        }
    }

    pub fn populate_reverse_dependencies(&mut self) {
        let keys: Vec<Id> = self.assets.keys().cloned().collect();
        for asset_id in keys.iter() {
            let (id, asset) = match self.assets.remove_entry(asset_id) {
                Some(e) => e,
                None => continue,
            };
            for dep_id in &asset.dependencies {
                let (id, mut dep) = match self.assets.remove_entry(dep_id) {
                    Some(e) => e,
                    None => continue,
                };
                dep.dependents.insert(asset_id.clone());
                self.assets.insert(id, dep);
            }
            self.assets.insert(id, asset);
        }
    }

    pub fn roots(&self) -> &HashSet<PathBuf> {
        &self.roots
    }

    pub fn loc_roots(&self) -> impl Iterator<Item = &PathBuf> {
        self.loc_roots.iter()
    }

    pub fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.assets.values()
    }

    pub fn asset(&self, id: &Id) -> Option<&Asset> {
        self.assets.get(id)
    }
}