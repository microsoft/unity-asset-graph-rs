use asset_graph_rs::database::Database;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: {} <root_path>", &args[0]);
        std::process::exit(1);
    }

    let mut db = match Database::new(&args[1]) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error initializing database: {}", e);
            std::process::exit(1);
        }
    };

    match db.populate() {
        Ok(_) => println!("DB populated with {} assets", db.assets().count()),
        Err(e) => {
            eprintln!("Error populating database: {}", e);
            std::process::exit(1);
        }
    }

    let file = std::fs::File::create("db.json").expect("Failed to create db.json");
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, &db).expect("Failed to write database to db.json");
}
