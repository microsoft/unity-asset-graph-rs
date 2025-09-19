use std::{
    collections::{HashSet, HashMap },
    path::PathBuf,
    fs,
};
use serde::{Deserialize, Serialize};
use crate::{
    asset::Asset,
    asset_type::AssetType,
    parser::ParseError,
    id::Id,
};

mod roots;
mod populate_pass1;
mod populate_pass2;
mod populate_pass3;

#[derive(Debug)]
pub struct DatabaseError {
    message: String,
    inner: Option<ParseError>,
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

    pub fn populate(&mut self) -> Result<(), DatabaseError> {
        self.populate_pass1_find()?;
        let broker = self.populate_pass2_resolve()?;
        self.populate_pass3_link(broker)?;
        self.populate_reverse_dependencies();
        Ok(())
    }

    pub fn populate_reverse_dependencies(&mut self) {
        // loop over a copy of the keys, and take the assets out of the map while we do this
        // so we can mutate them
        let keys: Vec<Id> = self.assets.keys().cloned().collect();
        for asset_id in keys.iter() {
            let (asset_id, asset) = match self.assets.remove_entry(asset_id) {
                Some(e) => e,
                None => continue,
            };

            // loop over the asset's (forward) relations
            for relation in asset.relations.iter() {
                // take the related asset out of the map too
                let (rel_id, mut rel_asset) = match self.assets.remove_entry(relation.id()) {
                    Some(e) => e,
                    None => {
                        let a = Asset {
                            id: relation.id().clone(),
                            asset_type: AssetType::BrokenRef,
                            ..Default::default()
                        };
                        (relation.id().clone(), a)
                    },
                };
                // add the back relation to the related asset
                rel_asset.back_relations.insert(asset.invert_relation(relation));
                self.assets.insert(rel_id, rel_asset);
            }
            self.assets.insert(asset_id, asset);
        }
    }

    pub fn roots(&self) -> &HashSet<PathBuf> {
        &self.roots
    }

    pub fn loc_roots(&self) -> impl Iterator<Item = &PathBuf> {
        self.loc_roots.iter()
    }

    pub fn asset(&self, id: &Id) -> Option<&Asset> {
        self.assets.get(id)
    }

    pub fn asset_mut(&mut self, id: &Id) -> Option<&mut Asset> {
        self.assets.get_mut(id)
    }

    pub fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.assets.values()
    }

    pub fn assets_by_name(&self, name: &str) -> impl Iterator<Item = &Asset> {
        self.assets.values().filter(move |a| {
            if let Some(p) = a.path.as_ref()
                && let Some(file_name) = p.file_name()
                && let Some(name_str) = file_name.to_str()
                && name_str == name {
                true
            }
            else if let Id::CsType { name: n, .. } = &a.id && n == name {
                return true;
            }
            else {
                false
            }
        })
    }
}