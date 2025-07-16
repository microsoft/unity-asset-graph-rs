use clap::{
    command,
    Parser,
    Subcommand,
    arg
};
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
};
use uuid::Uuid;
use asset_graph_rs::{
    asset::AssetType,
    database::Database,
    id::Id,
    version::DatabaseFile,
};

#[derive(Parser)]
struct CliArgs {
    #[command(subcommand)]
    command: CliCommand,
    #[arg(long, short = 'd', default_value = "db.bin")]
    db_path: String,
}

#[derive(Subcommand)]
enum CliCommand {
    FindAssets {
        #[arg(long, short = 'p')]
        root_path: String,
        #[arg(long, short = 'r', default_value = None)]
        relative_to: Option<String>,
    },
    ResolveAssets,
    Info {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    FindUnused {
        #[arg(long)]
        id_type: Option<OrphanFilter>,
        #[arg(long, default_value = "false")]
        id_only: bool,
    },
    FindBrokenRefs {
        #[arg(long)]
        id_type: Option<OrphanFilter>,
        #[arg(long, default_value = "false")]
        id_only: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OrphanFilter {
    Guid,
    Loc,
}

impl From<String> for OrphanFilter {
    fn from(value: String) -> Self {
        if value.eq_ignore_ascii_case("guid") {
            OrphanFilter::Guid
        } else if value.eq_ignore_ascii_case("loc") {
            OrphanFilter::Loc
        } else {
            panic!("Invalid orphan filter type: {}", value);
        }
    }
}

fn main() {
    let args = CliArgs::parse();
    match args.command {
        CliCommand::FindAssets { root_path, relative_to } => {
            find_assets(args.db_path, root_path, relative_to);
        },
        CliCommand::ResolveAssets => {
            resolve_assets(args.db_path);
        },
        CliCommand::Info { id, name } => {
            info(&args.db_path, id, name);
        },
        CliCommand::FindUnused { id_type, id_only } => {
            find_unused(&args.db_path, id_type, id_only);
        },
        CliCommand::FindBrokenRefs { id_type, id_only } => {
            find_broken_refs(&args.db_path, id_type, id_only);
        },
    }
}

fn find_assets(db_path: String, root_path: String, relative_to: Option<String>) {
    let mut db = match Database::new(&root_path, relative_to.as_deref()) {
        Ok(db) => db,
        Err(e) => {
            panic!("Error initializing database: {}", e);
        }
    };

    if let Err(e) = db.find_assets() {
        panic!("Error finding assets: {}", e);
    }

    let mut file = File::create(&db_path)
        .expect(format!("Failed to create {db_path}").as_str());
    let bin = rmp_serde::to_vec(&DatabaseFile::from(db))
        .expect("Failed to serialize database");
    file.write_all(&bin)
        .expect(format!("Failed to write database to {db_path}").as_str());
}

fn resolve_assets(db_path: String) {
    let file = File::open(&db_path)
        .expect(format!("Failed to open {db_path}").as_str());
    let db: DatabaseFile = match rmp_serde::from_read(file) {
        Ok(db) => {
            println!("Loaded database from {}", db_path);
            db
        },
        Err(_) => {
            panic!("Error reading database from {}", db_path);
        }
    };
    let mut db = db.database;

    if let Err(e) = db.resolve_assets() {
        panic!("Error resolving assets: {}", e);
    }

    let mut file = File::create(&db_path)
        .expect(format!("Failed to create {db_path}").as_str());
    let bin = rmp_serde::to_vec(&DatabaseFile::from(db))
        .expect("Failed to serialize database");
    file.write_all(&bin)
        .expect(format!("Failed to write database to {db_path}").as_str());
}

fn info(db_path: &str, id: Option<String>, name: Option<String>) {
    let file = File::open(&db_path)
        .expect(format!("Failed to open {db_path}").as_str());
    let mut db: Database = match rmp_serde::from_read(file) {
        Ok(db) => {
            println!("Loaded database from {}", db_path);
            db
        },
        Err(_) => {
            panic!("Error reading database from {}", db_path);
        }
    };
    db.populate_reverse_dependencies();

    if let Some(id) = id.as_ref() {
        let asset = if let Ok(id) = Uuid::parse_str(&id) {
            db.asset(&Id::Guid(id.clone()))
        }
        else {
            db.asset(&Id::Loc(id.clone()))
        };

        match asset {
            None => {
                panic!("No asset found with ID: {}", id);
            },
            Some(asset) => {
                println!("{}", asset.bind(&db));
            },
        };
    }
    else if let Some(name) = name {
        if let Some(asset) = db.asset_by_name(&name) {
            println!("{}", asset.bind(&db));
        } else {
            panic!("No asset found with name: {}", name);
        }
    }
    else {
        panic!("Either --id or --name must be provided");
    }
    
}

fn find_unused(db_path: &str, id_type: Option<OrphanFilter>, id_only: bool) {
    let file = File::open(&db_path)
        .expect(format!("Failed to open {db_path}").as_str());
    let mut db: Database = match rmp_serde::from_read(file) {
        Ok(db) => {
            println!("Loaded database from {}", db_path);
            db
        },
        Err(_) => {
            panic!("Error reading database from {}", db_path);
        }
    };
    
    db.populate_reverse_dependencies();

    let mut orphans = HashMap::new();
    let mut broken_refs = HashMap::new();
    for asset in db.assets() {
        if let Some(id_type) = id_type {
            if id_type == OrphanFilter::Guid && let Id::Loc(_) = asset.id {
                continue;
            }
            if id_type == OrphanFilter::Loc && let Id::Guid(_) = asset.id {
                continue;
            }
        }

        if asset.dependents.len() == 0 {
            orphans.insert(asset.id.clone(), asset);
        }
        if asset.asset_type == AssetType::BrokenRef {
            broken_refs.insert(asset.id.clone(), asset);
        }
    }

    println!("Unused assets ({}):", orphans.len());
    for asset in orphans.values() {
        if id_only {
            println!("{}", asset.id);
        }
        else {
            println!("{}", asset.bind(&db).indent());
        }
    }
    if orphans.is_empty() {
        println!("No unused assets found.");
    }
}

fn find_broken_refs(db_path: &str, id_type: Option<OrphanFilter>, id_only: bool) {
    let file = File::open(&db_path)
        .expect(format!("Failed to open {db_path}").as_str());
    let mut db: Database = match rmp_serde::from_read(file) {
        Ok(db) => {
            println!("Loaded database from {}", db_path);
            db
        },
        Err(_) => {
            panic!("Error reading database from {}", db_path);
        }
    };
    
    db.populate_reverse_dependencies();

    let mut broken_refs = HashMap::new();
    for asset in db.assets() {
        if let Some(id_type) = id_type {
            if id_type == OrphanFilter::Guid && let Id::Loc(_) = asset.id {
                continue;
            }
            if id_type == OrphanFilter::Loc && let Id::Guid(_) = asset.id {
                continue;
            }
        }

        if asset.asset_type == AssetType::BrokenRef {
            broken_refs.insert(asset.id.clone(), asset);
        }
    }

    println!("\nBroken references ({}):", broken_refs.len());
    for asset in broken_refs.values() {
        if id_only {
            println!("{}", asset.id);
        }
        else {
            println!("{}", asset.bind(&db).indent());
        }
    }
    if broken_refs.is_empty() {
        println!("No broken references found.");
    }
}