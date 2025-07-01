use std::{
    collections::{HashSet, HashMap },
    path::PathBuf,
};
use serde::Serialize;
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

#[derive(Serialize)]
pub struct Database {
    roots: HashSet<PathBuf>,
    assets: HashMap<Id, Asset>,
}

impl Database {
    pub fn new(root: &str) -> Result<Self, DatabaseError> {
        let mut db = Self {
            roots: HashSet::new(),
            assets: HashMap::new(),
        };

        match db.add_root_str(root) {
            Ok(_) => Ok(db),
            Err(e) => Err(e),
        }
    }

    pub fn populate(&mut self) -> Result<(), DatabaseError> {
        self.find_assets()
    }

    pub fn roots(&self) -> &HashSet<PathBuf> {
        &self.roots
    }

    pub fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.assets.values()
    }

    pub fn asset(&self, id: &Id) -> Option<&Asset> {
        self.assets.get(id)
    }
}