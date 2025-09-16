use std::{
    mem,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};
use crate::{
    asset::Asset,
    asset_type::AssetType,
    database::{Database, DatabaseError},
    parser::{self, ParseError},
    util,
};

const THREADS: usize = 4;

impl Database {
    pub fn find_assets(&mut self) -> Result<(), DatabaseError> {
        let mut paths = Vec::new();
        for root in &self.roots {
            paths.push(root.clone());
        }
        let paths = Arc::new(Mutex::new(paths));
        let (tx, rx) = mpsc::channel();
        let (err_tx, err_rx) = mpsc::channel();
        let mut handles = vec![];

        for _ in 0..THREADS {
            let paths = Arc::clone(&paths);
            let tx = tx.clone();
            let err_tx = err_tx.clone();
            let relative_to = self.relative_to.clone();
            handles.push(thread::spawn(move || {
                Self::find_assets_job(paths, relative_to.as_ref(), tx, err_tx);
            }));
        }

        loop {
            while let Ok(asset) = rx.try_recv() {
                self.assets.insert(asset.id.clone(), asset);
                print!("\rFinding assets: {}", self.assets.len());
            }

            let mut first = true;
            while let Ok(e) = err_rx.try_recv() {
                if let Some(&ParseError { ref path, .. }) = e.inner.as_ref() && !self.roots.contains(path) {
                    if first {
                        eprintln!();
                        first = false;
                    }
                    eprintln!("Error finding asset: {}", e);
                }
            }

            if handles.iter().all(|h| h.is_finished()) {
                println!();
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        handles.into_iter().for_each(|h| {
            if let Err(e) = h.join() {
                eprintln!("Error joining thread: {:?}", e);
            }
        });

        self.resolve_assets()?;

        println!("\nFound {} assets in {} roots", self.assets.len(), self.roots.len());
        Ok(())
    }

    fn find_assets_job(
        paths: Arc<Mutex<Vec<PathBuf>>>,
        relative_to: Option<&PathBuf>,
        assets_tx: mpsc::Sender<Asset>,
        err_tx: mpsc::Sender<DatabaseError>,
    ) {
        let mut retries = 0usize;
        while retries < 3 {
            let path = match paths.lock().unwrap().pop() {
                Some(p) => {
                    retries = 0;
                    match relative_to {
                        Some(rel) => rel.join(p),
                        None => p,
                    }
                },
                None => {
                    retries += 1;
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };

            if !path.exists() {
                let err = DatabaseError {
                    message: format!("Asset path '{}' does not exist", path.display()),
                    inner: None,
                };
                if let Err(e) = err_tx.send(err) {
                    eprintln!("Error sending error: {}", e);
                    continue;
                }
            }

            // skip non-asset files/folders
            if let Some(file_name) = path.file_name()
                && let Some(name) = file_name.to_str()
                && name.ends_with("~") {
                continue;
            }

            match Self::find_assets_file(&path, relative_to) {
                Ok(Some(asset)) => {
                    if let Err(e) = assets_tx.send(asset) {
                        eprintln!("Error sending asset: {}", e);
                    }
                },
                Err(e) => {
                    if let Err(e) = err_tx.send(e) {
                        eprintln!("Error sending error: {}", e);
                    }
                },
                _ => { },
            }

            if path.is_dir() {
                match Self::find_assets_dir(&path) {
                    Ok(new_paths) => {
                        match paths.lock() {
                            Ok(mut paths) => {
                                for p in new_paths {
                                    paths.push(p);
                                }
                            },
                            Err(e) => {
                                eprintln!("Error locking paths: {}", e);
                                continue;
                            }
                        }
                    },
                    Err(e) => {
                        if let Err(e) = err_tx.send(e) {
                            eprintln!("Error sending error: {}", e);
                        }
                    }
                };
            }
        }
    }

    fn find_assets_dir(path: &PathBuf) -> Result<Vec<PathBuf>, DatabaseError>{
        let files = match path.read_dir() {
            Ok(files) => files,
            Err(e) => return Err(DatabaseError { message: format!("Failed to read directory '{}': {}", path.display(), e), inner: None }),
        };

        let mut paths = vec![];
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

            paths.push(path);
        }

        Ok(paths)
    }

    fn find_assets_file(path: &PathBuf, relative_to: Option<&PathBuf>) -> Result<Option<Asset>, DatabaseError> {
        let asset_guid = match util::get_id_of_asset(path) {
            Ok(id) => id,
            Err(_) => return Ok(None),
        };

        let rel_path = if let Some(rel_to) = relative_to.as_ref()
            && let Ok(rel) = path.strip_prefix(rel_to) {
            PathBuf::from(rel)
        }
        else {
            path.clone()
        };

        let asset = Asset {
            id: asset_guid,
            asset_type: if path.is_dir() {
                AssetType::Directory
            }
            else {
                (&rel_path).into()
            },
            path: Some(rel_path),
            ..Default::default()
        };
        
        Ok(Some(asset))
    }

    pub fn resolve_assets(&mut self) -> Result<(), DatabaseError> {
        let asset_count = self.assets.len();
        let assets: Arc<Mutex<Vec<Asset>>> = Arc::new(Mutex::new(
            mem::take(&mut self.assets)
            .into_values()
            .filter(|a| a.path.is_some())
            .collect()));

        let (tx, rx) = mpsc::channel();
        let (err_tx, err_rx) = mpsc::channel();
        let mut handles = vec![];

        for _ in 0..THREADS {
            let assets = Arc::clone(&assets);
            let tx = tx.clone();
            let err_tx = err_tx.clone();
            let relative_to = self.relative_to.clone();
            handles.push(thread::spawn(move || {
                Self::resolve_assets_job(assets, relative_to.as_ref(), tx, err_tx);
            }));
        }

        loop {
            while let Ok(asset) = rx.try_recv() {
                self.assets.insert(asset.id.clone(), asset);

                let progress = asset_count - assets.lock().unwrap().len();
                let pct = (progress as f64 / asset_count as f64) * 100.0;
                print!("\rResolving assets: {:.2}% ({}/{})", pct, progress, asset_count);
            }

            let mut first = true;
            while let Ok(e) = err_rx.try_recv() {
                if first {
                    eprintln!();
                    first = false;
                }
                eprintln!("Error resolving asset: {}", e);
            }

            if handles.iter().all(|h| h.is_finished()) {
                println!();
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    fn resolve_assets_job(
        assets: Arc<Mutex<Vec<Asset>>>,
        relative_to: Option<&PathBuf>,
        tx: mpsc::Sender<Asset>,
        err_tx: mpsc::Sender<DatabaseError>,
    ) {
        let mut retries = 0usize;
        while retries < 3 {
            let mut asset = match assets.lock().unwrap().pop() {
                Some(a) => {
                    retries = 0;
                    a
                },
                None => {
                    retries += 1;
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };

            match parser::parse(&mut asset, relative_to) {
                Ok(subs) => {
                    if let Err(e) = tx.send(asset) {
                        eprintln!("Error sending asset: {}", e);
                    }
                    for asset in subs {
                        if let Err(e) = tx.send(asset) {
                            eprintln!("Error sending asset: {}", e);
                        }
                    }
                }
                Err(e) => {
                    let err = DatabaseError {
                        message: format!("Error parsing asset '{}': {}", asset.path.unwrap().display(), e),
                        inner: Some(e),
                    };
                    if let Err(e) = err_tx.send(err) {
                        eprintln!("Error sending error: {}", e);
                        continue;
                    }
                },
            };

            thread::yield_now();
        }
    }
}