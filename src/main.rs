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
use asset_graph_rs::{ AssetType, Database, DatabaseFile, Id, Relation };

#[derive(Parser)]
struct CliArgs {
    #[command(subcommand)]
    command: CliCommand,
    #[arg(long, short = 'd', default_value = "db.bin", help = "Path to the database file (default: db.bin)")]
    db_path: String,
}

#[derive(Subcommand)]
enum CliCommand {
    #[command(about = "Find assets in a Unity project directory and create a database file")]
    FindAssets {
        #[arg(long, short = 'p', help = "Path to the directory containing a Unity project")]
        root_path: String,
        #[arg(long, short = 'r', default_value = None, help = "If supplied, make paths in the database relative to this path")]
        relative_to: Option<String>,
    },
    #[command(about = "Get information about a specific asset by ID or name")]
    Info {
        #[arg(long, help = "GUID of the asset")]
        guid: Option<Uuid>,
        #[arg(long, help = "Loc ID of the asset")]
        loc: Option<String>,
        #[arg(long, help = "C# declaration name of the asset")]
        cs: Option<String>,
        #[arg(long, help = "Name of the asset")]
        name: Option<String>,
        #[arg(long, help = "Show the list of detected package roots")]
        roots: bool,
    },
    #[command(about = "Find unused assets in the database")]
    FindUnused {
        #[arg(long, help = "Filter by ID type: 'guid' or 'loc'")]
        id_type: Option<OrphanFilter>,
        #[arg(long, default_value = "false", help = "Only print IDs of unused assets")]
        id_only: bool,
        #[arg(long, default_value = "false", help = "Only print totals")]
        summarize: bool,
    },
    #[command(about = "Find broken references in the database")]
    FindBrokenRefs {
        #[arg(long, help = "Filter by ID type: 'guid' or 'loc'")]
        id_type: Option<OrphanFilter>,
        #[arg(long, default_value = "false", help = "If true, only print IDs of broken references")]
        id_only: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OrphanFilter {
    UnityGuid,
    Loc,
    CsDeclaration,
}

impl OrphanFilter {
    pub fn matches(&self, id: &Id) -> bool {
        match self {
            OrphanFilter::UnityGuid => match id {
                Id::Guid(_) => true,
                _ => false,
            },
            OrphanFilter::Loc => match id {
                Id::Loc(_) => true,
                _ => false,
            },
            OrphanFilter::CsDeclaration => match id {
                Id::CsType { .. } => true,
                _ => false,
            },
        }
    }
}

impl From<String> for OrphanFilter {
    fn from(value: String) -> Self {
        if value.eq_ignore_ascii_case("guid") {
            OrphanFilter::UnityGuid
        } else if value.eq_ignore_ascii_case("loc") {
            OrphanFilter::Loc
        } else {
            panic!("Invalid orphan filter type: {}", value);
        }
    }
}

impl From<&Id> for OrphanFilter {
    fn from(value: &Id) -> Self {
        match value {
            Id::None => panic!("Cannot convert Id::None to OrphanFilter"),
            Id::Guid(_) => Self::UnityGuid,
            Id::Loc(_) => Self::Loc,
            Id::CsType { .. } => Self::CsDeclaration,
        }
    }
}

impl std::fmt::Display for OrphanFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnityGuid => write!(f, "Unity object"),
            Self::Loc => write!(f, "Localized string"),
            Self::CsDeclaration => write!(f, "C# declaration"),
        }
    }
}

fn main() {
    let args = CliArgs::parse();
    match args.command {
        CliCommand::FindAssets { root_path, relative_to } => {
            find_assets(args.db_path, root_path, relative_to);
        },
        CliCommand::Info { guid, loc, cs, name, roots } => {
            info(&args.db_path, guid, loc, cs, name, roots);
        },
        CliCommand::FindUnused { id_type, id_only, summarize} => {
            find_unused(&args.db_path, id_type, id_only, summarize);
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

    if let Err(e) = db.populate() {
        panic!("Error finding assets: {}", e);
    }

    let mut file = File::create(&db_path)
        .expect(format!("Failed to create {db_path}").as_str());
    let bin = rmp_serde::to_vec(&DatabaseFile::from(db))
        .expect("Failed to serialize database");
    file.write_all(&bin)
        .expect(format!("Failed to write database to {db_path}").as_str());
}

fn info(db_path: &str, guid: Option<Uuid>, loc: Option<String>, cs: Option<String>, name: Option<String>, roots: bool) {
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

    if roots {
        let mut sorted_roots: Vec<String> = db.roots().iter().map(|r| r.display().to_string()).collect();
        sorted_roots.sort();
        for r in &sorted_roots {
            println!("- {r}");
        }
    }
    else if guid.is_some() || loc.is_some() || cs.is_some() {
        let id = if let Some(guid) = guid {
            Id::Guid(guid)
        } else if let Some(loc) = loc {
            Id::Loc(loc)
        } else if let Some(cs) = cs {
            match cs.rsplit_once('.') {
                Some((namespace, name)) => Id::CsType { name: name.into(), namespace: Some(namespace.into()) },
                None => Id::CsType { name: cs, namespace: None },
            }
        } else {
            panic!("One of --guid, --loc, or --cs must be provided");
        };
        
        let asset = db.asset(&id);
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
        let mut count = 0;
        for asset in db.assets_by_name(&name) {
            count += 1;
            println!("{}", asset.bind(&db));
        }
        if count == 0 {
            panic!("No asset found with name: {}", name);
        }
    }
    else {
        panic!("One of --name, --guid, --loc, or --cs must be provided");
    }
    
}

fn find_unused(db_path: &str, id_type: Option<OrphanFilter>, id_only: bool, summarize: bool) {
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
    let mut types: HashMap<OrphanFilter, usize> = HashMap::new();
    for asset in db.assets() {
        if let Some(id_type) = id_type && !id_type.matches(&asset.id) {
            continue;
        }

        if asset.relations_iter().all(|r| !matches!(r, Relation::UsedBy(_))) {
            orphans.insert(asset.id.clone(), asset);

            let type_class: OrphanFilter = (&asset.id).into();
            let count = types.get(&type_class).unwrap_or(&0);
            types.insert(type_class, count + 1);
        }
    }

    println!("Unused assets ({}):", orphans.len());
    if summarize {
        for (t, count) in &types {
            println!("  {t}: {count}");
        }
    }
    else {
        for asset in orphans.values() {
            if id_only {
                println!("{}", asset.id);
            }
            else {
                println!("{}", asset.bind(&db).indent());
            }
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
            if id_type == OrphanFilter::UnityGuid && let Id::Loc(_) = asset.id {
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