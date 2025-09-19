use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    io::BufRead,
};
use crate::{
    asset::{Asset, Relation},
    asset_type::AssetType,
    id::Id,
    parser::ParseError,
    util,
};

pub fn parse(asset: &mut Asset, relative_to: Option<&PathBuf>) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(asset.path.as_ref().unwrap()),
        None => asset.path.as_ref().unwrap(),
    };

    let mut reader = match util::read_file_no_bom(path) {
        Ok(file) => file,
        Err(e) => return Err(ParseError::new(path, format!("Failed to read prefab file: {}", e))),
    };

    parse_reader(&mut reader, asset, relative_to)
}

fn parse_reader(
    reader: &mut dyn BufRead, 
    asset: &mut Asset, 
    relative_to: Option<&PathBuf>,
) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(asset.path.as_ref().unwrap()),
        None => asset.path.as_ref().unwrap(),
    };
    let locstrings: HashMap<String, String> = match serde_json::from_reader(reader) {
        Ok(map) => map,
        Err(e) => return Err(ParseError::new(path, format!("Failed to parse loc resource JSON: {}", e))),
    };

    // Use the parsed locstrings to create Asset instances
    let assets: Vec<Asset> = locstrings.keys().map(|key| {
        Asset {
            id: Id::Loc(key.clone()),
            asset_type: AssetType::LocString,
            relations: HashSet::from([Relation::ContainedBy(asset.id.clone())]),
            ..Default::default()
        }
    }).collect();

    Ok(assets)
}