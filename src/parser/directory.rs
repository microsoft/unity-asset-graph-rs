use std::{
    path::PathBuf,
};
use crate::{
    parser::ParseError,
    util,
    Asset,
    Relation,
};

pub fn parse(asset: &mut Asset, relative_to: Option<&PathBuf>) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(asset.path.as_ref().unwrap()),
        None => asset.path.as_ref().unwrap(),
    };

    if let Some(p) = path.parent() && let Ok(id) = util::get_id_of_asset(p) {
        asset.relations.insert(Relation::ContainedBy(id));
    }

    Ok(vec![])
}