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
    
    println!("Database initialized with roots:");
    for root in db.roots().iter() {
        println!("{}", root.display());
    }

    match db.populate() {
        Ok(_) => println!("DB populated with {} assets", db.assets().count()),
        Err(e) => {
            eprintln!("Error populating database: {}", e);
            std::process::exit(1);
        }
    }
}
