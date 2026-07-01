use crate::{
    Asset, Database, DatabaseError,
    parser::{self, TypeBroker},
};
use std::{
    mem,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

const THREADS: usize = 4;

impl Database {
    pub fn populate_pass2_resolve(&mut self) -> Result<TypeBroker, DatabaseError> {
        let asset_count = self.assets.len();
        let assets: Arc<Mutex<Vec<Asset>>> = Arc::new(Mutex::new(
            mem::take(&mut self.assets)
                .into_values()
                .filter(|a| a.path.is_some())
                .collect(),
        ));

        let broker = Arc::new(Mutex::new(TypeBroker::new()));

        let (tx, rx) = mpsc::channel();
        let (err_tx, err_rx) = mpsc::channel();
        let mut handles = vec![];

        for _ in 0..THREADS {
            let assets = Arc::clone(&assets);
            let broker = Arc::clone(&broker);
            let tx = tx.clone();
            let err_tx = err_tx.clone();
            let relative_to = self.relative_to.clone();
            handles.push(thread::spawn(move || {
                Self::resolve_assets_job(assets, broker, relative_to.as_ref(), tx, err_tx);
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
        Ok(Arc::into_inner(broker).unwrap().into_inner().unwrap())
    }

    fn resolve_assets_job(
        assets: Arc<Mutex<Vec<Asset>>>,
        broker: Arc<Mutex<TypeBroker>>,
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
                }
                None => {
                    retries += 1;
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };

            match parser::parse(&mut asset, relative_to, &broker) {
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
                    let err = DatabaseError::parse(asset.path.unwrap(), "Error parsing asset");
                    if let Err(e) = err_tx.send(err) {
                        eprintln!("Error sending error: {}", e);
                        continue;
                    }
                }
            };

            thread::yield_now();
        }
    }
}
