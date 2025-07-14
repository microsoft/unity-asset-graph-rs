use clap::{
    command,
    Parser,
    Subcommand,
    arg
};
use std::{
    io::Write,
    fs::File,
};
use uuid::Uuid;
use asset_graph_rs::{
    database::Database,
    id::Id,
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
        id: Option<Uuid>,
        #[arg(long)]
        name: Option<String>,
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
        }
    }
}

fn find_assets(db_path: String, root_path: String, relative_to: Option<String>) {
    let mut db = match Database::new(&root_path, relative_to.as_deref()) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error initializing database: {}", e);
            std::process::exit(1);
        }
    };

    match db.find_assets() {
        Ok(_) => println!("DB populated with {} assets in {} roots", db.assets().count(), db.roots().len()),
        Err(e) => {
            eprintln!("Error populating database: {}", e);
            std::process::exit(1);
        }
    }

    let mut file = File::create(&db_path)
        .expect(format!("Failed to create {db_path}").as_str());
    let bin = rmp_serde::to_vec(&db)
        .expect("Failed to serialize database");
    file.write_all(&bin)
        .expect(format!("Failed to write database to {db_path}").as_str());
}

fn resolve_assets(db_path: String) {
    let file = File::open(&db_path)
        .expect(format!("Failed to open {db_path}").as_str());
    let mut db: Database = match rmp_serde::from_read(file) {
        Ok(db) => {
            println!("Loaded database from {}", db_path);
            db
        },
        Err(e) => {
            eprintln!("Error reading database from {}: {}", db_path, e);
            std::process::exit(1);
        }
    };

    db.resolve_assets();

    let mut file = File::create(&db_path)
        .expect(format!("Failed to create {db_path}").as_str());
    let bin = rmp_serde::to_vec(&db)
        .expect("Failed to serialize database");
    file.write_all(&bin)
        .expect(format!("Failed to write database to {db_path}").as_str());
}

fn info(db_path: &str, id: Option<Uuid>, name: Option<String>) {
    let file = File::open(&db_path)
        .expect(format!("Failed to open {db_path}").as_str());
    let mut db: Database = match rmp_serde::from_read(file) {
        Ok(db) => {
            println!("Loaded database from {}", db_path);
            db
        },
        Err(e) => {
            panic!("Error reading database from {}: {}", db_path, e);
        }
    };
    db.populate_reverse_dependencies();

    if let Some(id) = id {
        match db.asset(&Id::new_uuid(id)) {
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