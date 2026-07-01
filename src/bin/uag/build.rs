use crate::CliArgs;
use clap::Args;
use std::path::PathBuf;
use unity_asset_graph::{Database, DatabaseFile};

/// Find assets in a Unity project directory and create a database file
#[derive(Args)]
pub struct BuildArgs {
    /// Path to the directory containing a Unity project
    #[arg(long, short)]
    project_path: PathBuf,

    /// Make paths in the database relative to this path
    #[arg(long, short, default_value = ".")]
    relative_to: PathBuf,
}

impl BuildArgs {
    pub fn run(&self, CliArgs { db_path, .. }: &CliArgs) -> Result<(), Box<dyn std::error::Error>> {
        let Self {
            project_path,
            relative_to,
        } = self;
        let project_path = project_path.canonicalize()?;

        let mut db = Database::new(&project_path, relative_to)?;
        db.populate()?;
        DatabaseFile::from(db).save(db_path)
    }
}
