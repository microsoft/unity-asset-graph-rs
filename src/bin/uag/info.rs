use crate::CliArgs;
use clap::Args;
use unity_asset_graph::{AssetFilter, DatabaseFile};

/// Get information about specific assets by ID or name
///
/// Partial IDs and paths are regular expressions, so escape special symbols with a backslash. Path
/// separators are always a forward slash '/' regardless of platform.
#[derive(Args)]
pub struct InfoArgs {
    /// Partial ID of the asset
    #[arg(long, short)]
    id: Option<AssetFilter>,

    /// Partial path of the asset
    #[arg(long, short)]
    path: Option<AssetFilter>,

    /// Show the list of detected package roots and exit
    #[arg(long)]
    roots: bool,
}

impl InfoArgs {
    pub fn run(&self, CliArgs { db_path, .. }: &CliArgs) {
        let Self { id, path, roots } = self;
        let db = DatabaseFile::load(db_path)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to load database file from {db_path}: {e}",
                    db_path = db_path.display()
                )
            })
            .database;

        if *roots {
            let mut sorted_roots: Vec<String> = db.roots().iter().map(|r| r.display().to_string()).collect();
            sorted_roots.sort();
            for r in &sorted_roots {
                println!("- {r}");
            }
        } else if let Some(id) = id {
            let assets = db.find_assets_by_id(&id);
            if assets.len() == 0 {
                panic!("No assets found with id: {id}");
            } else {
                for a in assets {
                    println!("{}", a.display_full());
                }
            }
        } else if let Some(path) = path {
            let assets = db.find_assets_by_path(&path);
            if assets.len() == 0 {
                panic!("No assets found with path: {path}");
            } else {
                for a in assets {
                    println!("{}", a.display_full());
                }
            }
        } else {
            panic!("One of --id or --path must be provided");
        }
    }
}
