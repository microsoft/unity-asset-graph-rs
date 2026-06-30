use clap::{
    Parser,
    Subcommand,
};
use std::{
    collections::HashMap,
};
use unity_asset_graph::{Asset, AssetType, Database, DatabaseFile, Id, Relation, QualifiedName, BoundAsset};

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
    Build {
        #[arg(long, short = 'p', help = "Path to the directory containing a Unity project")]
        root_path: String,
        #[arg(long, short = 'r', default_value = ".", help = "If supplied, make paths in the database relative to this path")]
        relative_to: String,
    },
    #[command(about = "Get information about a specific asset by ID or name")]
    Info {
        #[arg(long, help = "Partial ID of the asset")]
        id: Option<String>,
        #[arg(long, help = "Partial path of the asset")]
        path: Option<String>,
        #[arg(long, help = "Show the list of detected package roots")]
        roots: bool,
    },
    #[command(about = "Find unused assets in the database")]
    Unused {
        #[arg(long, help = "Filter by ID type: 'guid' or 'loc'")]
        id_type: Option<OrphanFilter>,
        #[arg(long, default_value = "false", help = "Only print IDs of unused assets")]
        id_only: bool,
        #[arg(long, default_value = "false", help = "Only print totals")]
        summarize: bool,
    },
    #[command(about = "Find broken references in the database")]
    Broken {
        #[arg(long, help = "Filter by ID type: 'guid' or 'loc'")]
        id_type: Option<OrphanFilter>,
        #[arg(long, default_value = "false", help = "If true, only print IDs of broken references")]
        id_only: bool,
    },
    #[command(about = "Find references to assets outside of the given folders")]
    Outside {
        #[arg(long, short = 'i', help = "Search for scripts within this container asset")]
        id: Vec<String>,
        #[arg(long, short = 'p', help = "Search for scripts within this container asset")]
        path: Vec<String>,
        #[arg(long, short = 'x', help = "Ignore these paths")]
        ignore: Vec<String>,
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
        CliCommand::Build { root_path, relative_to } => {
            find_assets(args.db_path, root_path, Some(relative_to));
        },
        CliCommand::Info { id, path, roots } => {
            info(&args.db_path, id, path, roots);
        },
        CliCommand::Unused { id_type, id_only, summarize} => {
            find_unused(&args.db_path, id_type, id_only, summarize);
        },
        CliCommand::Broken { id_type, id_only } => {
            find_broken_refs(&args.db_path, id_type, id_only);
        },
        CliCommand::Outside { id, path, ignore } => {
            find_outside_refs(&args.db_path, id, path, ignore);
        }
    }
}

fn find_assets(db_path: String, root_path: String, relative_to: Option<String>) {
    let mut db = Database::new(&root_path, relative_to.as_deref()).expect("Error initializing database");

    if let Err(e) = db.populate() {
        panic!("Error finding assets: {}", e);
    }

    DatabaseFile::from(db).save(&db_path).expect("Error saving database file");
}

fn info(db_path: &str, id: Option<String>, path: Option<String>, roots: bool) {
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
    else if let Some(id) = id {
        let assets = db.find_assets_by_id(id.as_str()).expect("--id is not a valid regular expression");
        if assets.len() == 0 {
            panic!("No assets found with id: {id}");
        } else {
            for a in assets {
                println!("{}", a.bind(&db));
            }
        }
    }
    else if let Some(path) = path {
        let assets = db.find_assets_by_path(path.as_str()).expect("--path is not a valid regular expression");
        if assets.len() == 0 {
            panic!("No assets found with path: {path}");
        } else {
            for a in assets {
                println!("{a}");
            }
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
        if let Some(id_type) = id_type && !id_type.matches(&asset.id()) {
            continue;
        }

        if asset.asset().relations_iter().all(|r| !matches!(r, Relation::UsedBy(_))) {
            orphans.insert(asset.id().clone(), asset.clone());

            let type_class: OrphanFilter = asset.id().into();
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
                println!("{}", asset.id());
            }
            else {
                println!("{}", asset.clone().indent());
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
            if id_type == OrphanFilter::UnityGuid && let Id::Loc(_) = asset.id() {
                continue;
            }
            if id_type == OrphanFilter::Loc && let Id::Guid(_) = asset.id() {
                continue;
            }
        }

        if asset.asset_type() == &AssetType::BrokenRef {
            broken_refs.insert(asset.id().clone(), asset);
        }
    }

    println!("\nBroken references ({}):", broken_refs.len());
    for asset in broken_refs.values() {
        if id_only {
            println!("{}", asset.id());
        }
        else {
            println!("{}", asset.clone().indent());
        }
    }
    if broken_refs.is_empty() {
        println!("No broken references found.");
    }
}

fn find_outside_refs(db_path: &str, container_id: Vec<String>, container_path: Vec<String>, ignore_paths: Vec<String>) {
    let db = DatabaseFile::load(db_path)
        .expect(format!("Failed to load database file from {}", db_path).as_str())
        .database;

    let mut roots = vec![];
    for id in container_id {
        roots.extend(db.find_assets_by_id(&id)
            .expect("Supplied partial ID is not a valid regular expression")
            .map(|a| a.bind(&db)));
    }
    for path in container_path {
        roots.extend(db.find_assets_by_path(&path)
            .expect("Supplied partial path is not a valid regular expression"));
    }
    if roots.is_empty() {
        panic!("At least one container asset must be specified via --id or --path");
    }

    let mut inside = HashMap::new();
    for root in &roots {
        inside = find_all(&db, root.asset(), inside);
    }
    println!("In-group contains {} assets from {} containers", inside.len(), roots.len());

    let mut outside = HashMap::new();
    for asset in inside.values() {
        for relation in asset.relations_iter() {
            if let Relation::Uses(id) = relation
                && !inside.contains_key(id) {
                let a = db.asset(id).expect("Bad used-by ref!");
                let path = a.path();
                if ignore_paths.iter().all(|p| !path.starts_with(p)) {
                    outside.insert(id.clone(), (a, path));
                }
            }
        }
    }

    println!("Outside references ({}):", outside.len());
    for (outside, path) in outside.values() {
        println!("- {} ({})", &outside.id(), path.display());

        let users: Vec<BoundAsset> = outside.asset().relations_iter()
            .filter_map(|r| {
                if let Relation::UsedBy(id) = r && inside.contains_key(id) {
                    db.asset(id)
                } else {
                    None
                }
            }).collect();

        println!("  Used by: ({})", users.len());

        for user in users {
            println!("    {}, {}", user.id(), user.path().display());
        }

        println!();
    }
}

fn find_all<'a>(db: &'a Database, asset: &'a Asset, mut results: HashMap<Id, &'a Asset>) -> HashMap<Id, &'a Asset> {
    results.insert(asset.id.clone(), asset);
    for relation in asset.relations_iter() {
        if let Relation::Contains(other) = relation
            && let Some(other) = db.asset(other) {
            results = find_all(db, other, results);
        }
    }
    results
}

