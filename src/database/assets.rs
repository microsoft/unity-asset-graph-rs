use std::{
    collections::HashMap,
    io::BufRead,
    path::PathBuf,
    sync::{LazyLock, Arc, Mutex, mpsc},
    thread,
    time::Duration,
};
use regex::Regex;
use uuid::Uuid;
use crate::{
    asset::Asset,
    database::{Database, DatabaseError},
    id::Id,
    parser,
    util::read_file_no_bom
};

static META_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^guid: ([0-9a-f]{32})$").expect("Failed to compile meta id regex")
});

const THREADS: usize = 4;

impl Database {
    pub fn find_assets(&mut self) -> Result<(), DatabaseError> {
        let mut paths = Vec::new();
        for root in &self.roots {
            paths.push(root.clone());
        }
        let paths = Arc::new(Mutex::new(paths));
        let assets = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = vec![];

        for _ in 0..THREADS {
            let mut paths = Arc::clone(&paths);
            let mut assets = Arc::clone(&assets);
            let relative_to = self.relative_to.clone();
            handles.push(thread::spawn(move || {
                loop {
                    let path = match &paths.lock().unwrap().pop() {
                        Some(p) => p.clone(),
                        None => break,
                    };

                    Self::find_assets_job(&path, relative_to.as_ref(), &mut paths, &mut assets)
                        .unwrap_or_else(|e| eprintln!("Error finding assets in '{}': {}", path.display(), e));

                    thread::yield_now();
                }
            }));
        }

        loop {
            thread::sleep(Duration::from_secs(1));

            if let Ok(assets) = assets.lock() {
                print!("\rFinding assets: {}", assets.len());
            }

            if let Ok(paths) = paths.lock() {
                if paths.is_empty() {
                    println!("");
                    break;
                }
            }
        }

        handles.into_iter().for_each(|h| {
            if let Err(e) = h.join() {
                eprintln!("Error joining thread: {:?}", e);
            }
        });

        if let Ok(m) = Arc::try_unwrap(assets)
            && let Ok(assets) = m.into_inner()
        {
            self.assets = assets;
            println!("Found {} assets in {} roots", self.assets.len(), self.roots.len());
            Ok(())
        }
        else {
            Err(DatabaseError {
                message: "Failed to unwrap assets mutex".to_string(),
                inner: None,
            })
        }
    }

    pub fn resolve_assets(&mut self) -> Result<(), DatabaseError> {
        let paths: Arc<Mutex<Vec<(Id, PathBuf)>>> = Arc::new(Mutex::new(
            self.assets.values().filter_map(|a| {
                if let Id::Guid(_) = a.id {
                    Some((a.id.clone(), a.path.clone()))
                }
                else {
                    None
                }
            })
            .collect()
        ));
        let (tx, rx) = mpsc::channel();
        let mut handles = vec![];

        for _ in 0..THREADS {
            let paths = Arc::clone(&paths);
            let mut tx = tx.clone();
            let relative_to = self.relative_to.clone();
            handles.push(thread::spawn(move || {
                loop {
                    let (id, path) = match paths.lock().unwrap().pop() {
                        Some(p) => p,
                        None => break,
                    };

                    Self::resolve_assets_job(&id, &path, relative_to.as_ref(), &mut tx)
                        .unwrap_or_else(|e| eprintln!("Error finding assets in '{}': {}", path.display(), e));

                    thread::yield_now();
                }
            }));
        }

        let mut progress = 0usize;
        loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(asset) => {
                    self.assets.insert(asset.id.clone(), asset);
                    progress += 1;
                },
                Err(_) => break,
            };

            let pct = (progress as f64 / self.assets.len() as f64) * 100.0;
            print!("\rResolving assets: {:.2}% ({}/{})", pct, progress, self.assets.len());
        }

        handles.into_iter().for_each(|h| {
            if let Err(e) = h.join() {
                eprintln!("Error joining thread: {:?}", e);
            }
        });

        println!("Found {} assets in {} roots", self.assets.len(), self.roots.len());
        Ok(())
    }

    fn find_assets_job(
        path: &PathBuf, 
        relative_to: Option<&PathBuf>,
        paths: &mut Arc<Mutex<Vec<PathBuf>>>,
        assets: &mut Arc<Mutex<HashMap<Id, Asset>>>,
    ) -> Result<(), DatabaseError>{
        let path = match &relative_to {
            Some(rel) => &rel.join(path),
            None => path,
        };

        if !path.exists() {
            return Err(DatabaseError { message: format!("Asset path '{}' does not exist", path.display()), inner: None });
        }

        // skip non-asset files/folders
        if path.file_name().unwrap().to_str().unwrap().ends_with("~") {
            return Ok(());
        }

        if path.is_dir() {
            Self::find_assets_dir(path, paths)
        }
        else {
            Self::find_assets_file(path, relative_to, assets)
        }
    }

    fn find_assets_dir(path: &PathBuf, paths: &mut Arc<Mutex<Vec<PathBuf>>>) -> Result<(), DatabaseError>{
        let files = match path.read_dir() {
            Ok(files) => files,
            Err(e) => return Err(DatabaseError { message: format!("Failed to read directory '{}': {}", path.display(), e), inner: None }),
        };

        for f in files {
            let f = match f {
                Err(e) => {
                    eprintln!("Error reading file in '{}': {}", path.display(), e);
                    continue;
                },
                Ok(f) => f,
            };

            let path = f.path();

            // skip meta files for now
            if let Some(ext) = path.extension().and_then(|s| s.to_str())
                && ext == "meta" {
                continue;
            }

            match paths.lock() {
                Ok(mut paths) => {
                    paths.push(path);
                },
                Err(e) => {
                    eprintln!("Error acquiring lock on paths: {e}");
                    continue;
                },
            }
        }

        Ok(())
    }

    fn find_assets_file(path: &PathBuf, 
        relative_to: Option<&PathBuf>,
        assets: &mut Arc<Mutex<HashMap<Id, Asset>>>,
    ) -> Result<(), DatabaseError> {
        let meta_path = path.with_file_name(format!("{}.meta", path.file_name().unwrap().to_str().unwrap()));
        if !meta_path.exists() {
            return Ok(());
        }

        // read the meta file
        let meta_reader = match read_file_no_bom(&meta_path) {
            Ok(r) => r,
            Err(e) => return Err(DatabaseError {
                message: format!("failed to read meta file '{}'", meta_path.display()),
                inner: Some(Box::new(e)),
            }),
        };

        let mut asset_guid = None;
        for line in meta_reader.lines() {
            if let Ok(line) = line
                && let Some(captures) = META_REGEX.captures(&line)
                && let Some(m) = captures.get(1)
                && let Ok(uuid) = Uuid::parse_str(m.as_str()){
                // Extract the GUID from the meta file
                asset_guid = Some(uuid);
                break;
            }
        }
        let asset_guid = asset_guid.expect("Meta file must contain a valid GUID");

        let rel_path = if let Some(rel_to) = relative_to.as_ref()
            && let Ok(rel) = path.strip_prefix(rel_to) {
            PathBuf::from(rel)
        }
        else {
            path.clone()
        };

        let asset = Asset::new_with_path(Id::Guid(asset_guid), rel_path);

        match assets.lock() {
            Ok(mut assets) => {
                assets.insert(asset.id.clone(), asset);
                Ok(())
            },
            Err(_) => {
                Err(DatabaseError { message: format!("Failed to acquire lock on assets"), inner: None })
            },
        }
    }

    fn resolve_assets_job(id: &Id,
        path: &PathBuf,
        relative_to: Option<&PathBuf>,
        tx: &mut mpsc::Sender<Asset>,
    ) -> Result<(), DatabaseError> {
        let mut asset = Asset::new_with_path(id.clone(), path.clone());
        let subassets = match parser::parse(&mut asset, relative_to) {
            Ok(subs) => subs,
            Err(e) => {
                return Err(DatabaseError {
                    message: format!("Error parsing asset '{}': {}", path.display(), e),
                    inner: Some(Box::new(e)),
                });
            },
        };

        if let Err(e) = tx.send(asset) {
            return Err(DatabaseError {
                message: format!("Error sending asset '{}': {}", path.display(), e),
                inner: Some(Box::new(e)),
            });
        }

        for sub in subassets {
            if let Err(e) = tx.send(sub) {
                return Err(DatabaseError {
                    message: format!("Error sending subasset: {}", e),
                    inner: Some(Box::new(e)),
                });
            }
        }

        Ok(())
    }
}