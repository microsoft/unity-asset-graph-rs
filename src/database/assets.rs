use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
};
use crate::{
    asset::Asset, id::Id, parsers::meta::Meta, util::read_file_no_bom
};

use super::{Database, DatabaseError};

impl Database {
    pub fn find_assets(&mut self) -> Result<(), DatabaseError> {
        for root in self.roots.iter() {
            if let Err(e) = Self::find_assets_in_dir(root, &mut self.assets) {
                eprintln!("Error finding assets in '{}': {}", root.display(), e);
            }
        }
        Ok(())
    }

    fn find_assets_in_dir(path: &PathBuf, assets: &mut HashMap<Id, Asset>) -> Result<(), DatabaseError> {
        let dir = match fs::read_dir(path) {
            Ok(d) => d,
            Err(e) => {
                return Err(DatabaseError { message: format!("Error reading directory '{}': {}", path.display(), e), inner: None });
            },
        };
        for entry in dir {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Error reading entry in '{}': {}", path.display(), e);
                    continue;
                }
            };

            // skip non-meta files
            let meta_path = match entry.path().extension().and_then(|s| s.to_str()) {
                Some("meta") => entry.path(),
                _ => continue,
            };

            println!("Checking {}", meta_path.display());

            // read the meta file
            let meta_reader = match read_file_no_bom(&meta_path) {
                Ok(r) => r,
                Err(e) => return Err(DatabaseError {
                    message: format!("failed to read meta file '{}'", meta_path.display()),
                    inner: Some(Box::new(e)),
                }),
            };

            let meta_contents: Meta = match serde_yml::from_reader(meta_reader) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Error parsing .meta file '{}': {}", meta_path.display(), e);
                    continue;
                }
            };

            // process the asset file
            let asset_path = meta_path.with_extension("");
            if asset_path.is_dir() {
                // Recursively find assets in subdirectories
                if let Err(e) = Self::find_assets_in_dir(&asset_path, assets) {
                    eprintln!("Error finding assets in '{}': {}", asset_path.display(), e);
                }
            } else if asset_path.is_file() {
                // Process the file as an asset
                let asset = Asset::new(Id::Guid(meta_contents.guid), asset_path);
                assets.insert(asset.id.clone(), asset);
            }
        }

        Ok(())
    }
}