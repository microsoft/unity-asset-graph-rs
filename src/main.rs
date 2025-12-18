use clap::{
    command,
    Parser,
    Subcommand,
    arg
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    fs::File,
    io::Write,
};
use uuid::Uuid;
use asset_graph_rs::{Asset, AssetType, Database, DatabaseFile, Id, Relation };

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
        #[arg(long, help = "Path of the asset")]
        path: Option<String>,
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
    #[command(about = "Find references to assets outside of the folders")]
    FindOutsideRefs {
        #[arg(long, help = "Search for scripts within this container asset")]
        container_id: Option<Uuid>,
        #[arg(long, help = "Search for scripts within this container asset")]
        container_path: Option<String>,
        #[arg(long, help = "Ignore these paths")]
        ignore_paths: Vec<String>,
    }
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
        CliCommand::Info { guid, loc, cs, name, path, roots } => {
            info(&args.db_path, guid, loc, cs, name, path, roots);
        },
        CliCommand::FindUnused { id_type, id_only, summarize} => {
            find_unused(&args.db_path, id_type, id_only, summarize);
        },
        CliCommand::FindBrokenRefs { id_type, id_only } => {
            find_broken_refs(&args.db_path, id_type, id_only);
        },
        CliCommand::FindOutsideRefs { container_id, container_path, ignore_paths } => {
            find_outside_refs(&args.db_path, container_id, container_path, ignore_paths);
        }
    }
}

fn find_assets(db_path: String, root_path: String, relative_to: Option<String>) {
    let mut db = Database::new(&root_path, relative_to.as_deref()).expect("Error initializing database");

    if let Err(e) = db.populate() {
        panic!("Error finding assets: {}", e);
    }

    DatabaseFile::from(db).save(db_path).expect("Error saving database file");
}

fn info(db_path: &str, guid: Option<Uuid>, loc: Option<String>, cs: Option<String>, name: Option<String>, path: Option<String>, roots: bool) {
    let db = DatabaseFile::load(db_path)
        .expect(format!("Failed to load database file from {}", db_path).as_str())
        .database;

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
    else if let Some(path) = path {
        let pathbuf = Some(PathBuf::from(path));
        if let Some(a) = db.assets().find(|a| a.path.as_ref() == pathbuf.as_ref()) {
            println!("{}", a.bind(&db));
        } else {
            panic!("No asset found with path: {}", pathbuf.as_ref().unwrap().display());
        }
    }
    else {
        panic!("One of --name, --guid, --loc, or --cs must be provided");
    }
    
}

fn find_unused(db_path: &str, id_type: Option<OrphanFilter>, id_only: bool, summarize: bool) {
    let db = DatabaseFile::load(db_path)
        .expect(format!("Failed to load database file from {}", db_path).as_str())
        .database;

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
    let db = DatabaseFile::load(db_path)
        .expect(format!("Failed to load database file from {}", db_path).as_str())
        .database;

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

fn find_outside_refs(db_path: &str, container_id: Option<Uuid>, container_path: Option<String>, ignore_paths: Vec<String>) {
    let db = DatabaseFile::load(db_path)
        .expect(format!("Failed to load database file from {}", db_path).as_str())
        .database;

    let root = if let Some(id) = container_id {
        db.asset(&Id::Guid(id))
            .expect("Container asset with specified ID not found")
    } else if let Some(path) = container_path {
        let pathbuf = Some(PathBuf::from(path));
        if let Some(a) = db.assets().find(|a| a.path.as_ref() == pathbuf.as_ref()) {
            a
        } else {
            panic!("No asset found with path: {}", pathbuf.as_ref().unwrap().display());
        }
    } else {
        panic!("Either container_id or container_name must be provided");
    };

    let mut inside_scripts = HashSet::new();
    find_contained(&db, root, &|a| matches!(a.asset_type, AssetType::CsType), &mut inside_scripts);

    let mut outside_scripts = HashMap::new();
    for script_id in inside_scripts.iter() {
        let script_asset = db.asset(script_id).expect("Script asset not found");
        for relation in script_asset.relations_iter() {
            if let Relation::Uses(id @ Id::CsType { .. }) = relation
                && !inside_scripts.contains(id) {
                outside_scripts.insert(id.clone(), db.asset(id).expect("Outside script asset not found").clone());
            }
        }
    }

    for outside in outside_scripts.values_mut() {
        outside.relations = outside.relations.iter()
            .filter(|r| {
                if let Relation::UsedBy(id) = r {
                    inside_scripts.contains(id)
                } else {
                    false
                }
            })
            .cloned()
            .collect();
    }

    println!("Outside references ({}):", outside_scripts.len());
    for asset in outside_scripts.values() {
        let mut ignore = false;
        if !ignore_paths.is_empty() {
            if let Some(p) = &asset.path {
                for ip in &ignore_paths {
                    if p.display().to_string().contains(ip) {
                        ignore = true;
                        break;
                    }
                }
            }
        }
        if !ignore {
            println!("{}", asset.bind(&db).indent());
        }
    }
}

fn find_contained(db: &Database, asset: &Asset, condition: &impl Fn(&Asset) -> bool, results: &mut HashSet<Id>) {
    if condition(asset) {
        results.insert(asset.id.clone());
    }
    for relation in asset.relations_iter() {
        if let Relation::Contains(other) = relation {
            find_contained(db, db.asset(other).expect("Contained asset not found"), condition, results);
        }
    }
}