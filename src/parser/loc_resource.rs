use crate::{
    asset::{Asset, Relation},
    asset_type::AssetType,
    id::Id,
    parser::ParseError,
    util,
};
use std::{
    collections::{HashMap, HashSet}, io::BufRead, path::{Path, PathBuf},
};

pub fn parse(asset: &mut Asset, relative_to: &Path) -> Result<Vec<Asset>, ParseError> {
    let path = relative_to.join(asset.path.as_ref().unwrap());

    let mut reader = match util::read_file_no_bom(&path) {
        Ok(file) => file,
        Err(e) => {
            return Err(ParseError::new(path, format!("Failed to read prefab file: {}", e)));
        }
    };

    parse_reader(&mut reader, asset, relative_to)
}

fn parse_reader(
    reader: &mut dyn BufRead,
    asset: &mut Asset,
    relative_to: &Path,
) -> Result<Vec<Asset>, ParseError> {
    let path = relative_to.join(asset.path.as_ref().unwrap());
    let locstrings: HashMap<String, String> = match serde_json::from_reader(reader) {
        Ok(map) => map,
        Err(e) => {
            return Err(ParseError::new(
                path,
                format!("Failed to parse loc resource JSON: {}", e),
            ));
        }
    };

    // Use the parsed locstrings to create Asset instances
    let assets: Vec<Asset> = locstrings
        .keys()
        .map(|key| {
            Asset::new(
                Id::Loc(key.clone()),
                AssetType::LocString,
                None,
                [Relation::ContainedBy(asset.id.clone())],
            )
        })
        .collect();

    Ok(assets)
}
