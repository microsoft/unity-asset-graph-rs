use crate::CliArgs;
use clap::Args;
use std::collections::{HashMap, HashSet};
use unity_asset_graph::{AssetFilter, AssetType, DatabaseFile, Relation};

/// Find unused assets in the database
///
/// Partial IDs and paths are regular expressions, so escape special symbols with a backslash. Path
/// separators are always a forward slash '/' regardless of platform.
#[derive(Args)]
pub struct UnusedArgs {
    /// Only show assets whose IDs match this partial ID
    #[arg(long, short)]
    id: Vec<AssetFilter>,

    /// Only print IDs of unused assets
    #[arg(long)]
    id_only: bool,

    /// Only print totals
    #[arg(long)]
    summarize: bool,
}

impl UnusedArgs {
    pub fn run(&self, CliArgs { db_path, .. }: &CliArgs) -> Result<(), Box<dyn std::error::Error>> {
        let Self { id, id_only, summarize } = self;
        let db = DatabaseFile::load(db_path)?.database;

        let mut orphans = HashSet::new();
        let mut types: HashMap<AssetType, usize> = HashMap::new();

        for asset in &db {
            if (id.len() == 0 || id.iter().any(|id| asset.asset.id_matches(id)))
                && asset
                    .asset()
                    .relations_iter()
                    .all(|r| !matches!(r, Relation::UsedBy(_)))
            {
                types.entry(*asset.asset_type()).and_modify(|c| *c += 1).or_insert(1);
                orphans.insert(asset);
            }
        }

        println!("Unused assets ({}):", orphans.len());
        if *summarize {
            for (t, count) in &types {
                println!("  {t}: {count}");
            }
        } else {
            for asset in &orphans {
                if *id_only {
                    println!("{}", asset.id());
                } else {
                    println!("{}", asset.clone().indent().display_full());
                }
            }
        }
        if orphans.is_empty() {
            println!("No unused assets found.");
        }

        Ok(())
    }
}
