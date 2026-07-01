use crate::CliArgs;
use clap::Args;
use std::collections::HashSet;
use unity_asset_graph::{AssetFilter, AssetType, DatabaseFile};

/// Find broken references in the database
///
/// Partial IDs and paths are regular expressions, so escape special symbols with a backslash. Path
/// separators are always a forward slash '/' regardless of platform.
#[derive(Args)]
pub struct BrokenArgs {
    /// Only show assets that match this partial ID
    #[arg(long, short)]
    id: Vec<AssetFilter>,

    /// Only print IDs of broken references
    #[arg(long)]
    id_only: bool,
}

impl BrokenArgs {
    pub fn run(&self, CliArgs { db_path, .. }: &CliArgs) {
        let Self { id, id_only } = &self;

        let db = DatabaseFile::load(db_path)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to load database file from {db_path}: {e}",
                    db_path = db_path.display()
                )
            })
            .database;

        let mut broken_refs = HashSet::new();
        for asset in &db {
            if (id.len() == 0 || id.iter().any(|id| asset.asset.id_matches(id)))
                && asset.asset_type() == &AssetType::BrokenRef
            {
                broken_refs.insert(asset);
            }
        }

        println!("\nBroken references ({}):", broken_refs.len());
        for asset in &broken_refs {
            if *id_only {
                println!("{}", asset.id());
            } else {
                println!("{}", asset.clone().indent().display_full());
            }
        }
        if broken_refs.is_empty() {
            println!("No broken references found.");
        }
    }
}
