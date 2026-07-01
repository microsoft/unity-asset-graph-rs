use crate::{Asset, Relation, parser::ParseError, util};
use std::path::Path;

pub fn parse(asset: &mut Asset, relative_to: &Path) -> Result<Vec<Asset>, ParseError> {
    let path = relative_to.join(asset.path.as_ref().unwrap());

    if let Some(p) = path.parent()
        && let Ok(id) = util::get_id_of_asset(p)
    {
        asset.relations.insert(Relation::ContainedBy(id));
    }

    Ok(vec![])
}
