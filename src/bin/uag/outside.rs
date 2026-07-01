use crate::CliArgs;
use clap::{Args, CommandFactory, error::ErrorKind};
use std::collections::{HashMap, HashSet};
use unity_asset_graph::{AssetFilter, BoundAsset, BoundRelation, DatabaseFile, Id};

/// Show usages by in-group assets of out-group assets
///
/// Partial IDs and paths are regular expressions, so escape special symbols with a backslash. Path
/// separators are always a forward slash '/' regardless of platform.
#[derive(Args)]
pub struct OutsideArgs {
    /// Assets recursively contained by this partial ID are "in"
    #[arg(long)]
    in_id: Vec<AssetFilter>,

    /// Assets recursively contained by this partial path are "in"
    #[arg(long)]
    in_path: Vec<AssetFilter>,

    /// Only show out-group assets with this partial id
    #[arg(long)]
    out_id: Vec<AssetFilter>,

    /// Only show out-group assets with this partial path
    #[arg(long)]
    out_path: Vec<AssetFilter>,
}

impl OutsideArgs {
    pub fn run(&self, CliArgs { db_path, .. }: &CliArgs) -> Result<(), Box<dyn std::error::Error>> {
        let Self {
            in_id,
            in_path,
            out_id,
            out_path,
        } = self;

        if in_id.len() + in_path.len() == 0 {
            let e = CliArgs::command().error(ErrorKind::TooFewValues, "Must supply at least one of --in-id and --in-path");
            return Err(Box::new(e));
        }

        let db = DatabaseFile::load(db_path)?.database;

        let mut roots = HashSet::new();
        for id in in_id {
            roots.extend(db.find_assets_by_id(&id));
        }
        for path in in_path {
            roots.extend(db.find_assets_by_path(&path));
        }

        let root_len = roots.len();
        let mut inside = HashMap::new();
        for root in roots {
            inside = find_all(root, inside);
        }
        println!("In-group contains {} assets from {root_len} containers", inside.len());

        let mut outside = HashMap::new();
        for asset in inside.values() {
            for relation in asset.relations_iter() {
                if let BoundRelation::Uses(asset) = relation
                    && !inside.contains_key(asset.id())
                    && (out_id.len() == 0 || out_id.iter().any(|re| asset.asset().id_matches(re)))
                    && (out_path.len() == 0 || out_path.iter().any(|re| asset.path_matches(re)))
                {
                    outside.insert(asset.id().clone(), asset);
                }
            }
        }

        println!("Outside references ({}):", outside.len());
        for outside in outside.values() {
            println!(
                "{}",
                outside.display_full_filtered(|r| {
                    if let BoundRelation::UsedBy(a) = r
                        && inside.contains_key(a.id())
                    {
                        true
                    } else {
                        false
                    }
                })
            );
        }

        Ok(())
    }
}

fn find_all<'a>(asset: BoundAsset<'a>, mut results: HashMap<Id, BoundAsset<'a>>) -> HashMap<Id, BoundAsset<'a>> {
    for relation in asset.relations_iter() {
        if let BoundRelation::Contains(other) = relation {
            results = find_all(other, results);
        }
    }
    results.insert(asset.id().clone(), asset);
    results
}
