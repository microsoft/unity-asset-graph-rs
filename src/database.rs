use std::{
    collections::{
        HashSet,
        HashMap,
    },
    fs,
    io::BufReader,
    path::PathBuf,
};
use serde_json;
use crate::parsers::{
    manifest_json::ManifestJson,
    package_json::PackageJson,
};

#[derive(Debug)]
pub struct DatabaseError {
    message: String,
    inner: Option<Box<dyn std::error::Error>>,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(inner) = &self.inner {
            write!(f, "{}: {}", self.message, inner)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for DatabaseError {}

pub struct Database {
    // Database fields and methods
    roots: HashSet<PathBuf>,
}

impl Database {
    pub fn new(root: &str) -> Result<Self, DatabaseError> {
        let mut roots = HashSet::new();
        match Self::add_root_str(root, &mut roots) {
            Ok(_) => Ok(Self { roots }),
            Err(e) => Err(e),
        }
    }

    fn add_root_str(path: &str, roots: &mut HashSet<PathBuf>) -> Result<(), DatabaseError> {
        let abs_root = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(e) => return Err(DatabaseError {
                message: format!("failed to canonicalize path '{path}'"),
                inner: Some(Box::new(e)),
            }),
        };
        Self::add_root(abs_root, roots, &mut HashSet::new())
    }

    fn add_root(
        path: PathBuf, 
        roots: &mut HashSet<PathBuf>, 
        unresolved: &mut HashSet<String>
    ) -> Result<(), DatabaseError> {
        println!("{}", path.display());
        roots.insert(path.clone());

        // check for a manifest.json file
        let manifest_path = path.join("Packages").join("manifest.json");
        if manifest_path.exists() {
            let file = match fs::File::open(&manifest_path) {
                Ok(f) => f,
                Err(e) => return Err(DatabaseError {
                    message: format!("failed to open manifest file '{}'", manifest_path.display()),
                    inner: Some(Box::new(e)),
                }),
            };
            let reader = BufReader::new(file);
            let manifest: ManifestJson = match serde_json::from_reader(reader) {
                Ok(m) => m,
                Err(e) => return Err(DatabaseError {
                    message: format!("failed to parse manifest file '{}'", manifest_path.display()),
                    inner: Some(Box::new(e)),
                }),
            };

            for (name, version) in manifest.dependencies {
                if version.starts_with("file:") {
                    let dep_path = version.trim_start_matches("file:");
                    let dep_abs_path = path.join("Packages").join(dep_path.trim());

                    if roots.contains(&dep_abs_path) {
                        continue; // Already added
                    }
                    
                    if dep_abs_path.exists() {
                        Self::add_root(dep_abs_path, roots, unresolved)?;
                    } else {
                        eprintln!("Warning: Dependency path '{}' does not exist.", dep_abs_path.display());
                    }
                }
                else {
                    unresolved.insert(name);
                }
            }
        }

        // check for a package.json file
        let package_path = path.join("package.json");
        if package_path.exists() {
            let file = match fs::File::open(&package_path) {
                Ok(f) => f,
                Err(e) => return Err(DatabaseError {
                    message: format!("failed to open package file '{}'", package_path.display()),
                    inner: Some(Box::new(e)),
                }),
            };
            let reader = BufReader::new(file);
            let package: PackageJson = match serde_json::from_reader(reader) {
                Ok(p) => p,
                Err(e) => return Err(DatabaseError {
                    message: format!("failed to parse package file '{}'", package_path.display()),
                    inner: Some(Box::new(e)),
                }),
            };

            for (name, version) in package.dependencies.unwrap_or(HashMap::new()) {
                if version.starts_with("file:") {
                    let dep_path = version.trim_start_matches("file:");
                    let dep_abs_path = path.join(dep_path.trim());

                    if roots.contains(&dep_abs_path) {
                        continue; // Already added
                    }

                    if dep_abs_path.exists() {
                        Self::add_root(dep_abs_path, roots, unresolved)?;
                    } else {
                        eprintln!("Warning: Dependency path '{}' does not exist.", dep_abs_path.display());
                    }
                }
                else {
                    unresolved.insert(name);
                }
            }
        }

        // check for a Library/PackageCache directory
        let lib_path = path.join("Library").join("PackageCache");
        if lib_path.exists() {
            let dir = match fs::read_dir(&lib_path) {
                Ok(d) => d,
                Err(e) => return Err(DatabaseError {
                    message: format!("failed to read directory '{}'", lib_path.display()),
                    inner: Some(Box::new(e)),
                }),
            };
            for pkg in dir {
                let entry = match pkg {
                    Err(_) => continue,
                    Ok(e) => e,
                };
                
                let dep_path = entry.path();
                let name = match dep_path.file_name() {
                    None => continue,
                    Some(n) => match n.to_str() {
                        None => continue,
                        Some(s) => s.to_string(),
                    },
                };
                let pkg_end = match name.find('@') {
                    None => continue,
                    Some(idx) => idx,
                };
                let name = &name[..pkg_end];

                if dep_path.is_dir() && unresolved.contains(name) {
                    unresolved.remove(name);
                    if let Err(e) = Self::add_root(dep_path, roots, unresolved) {
                        eprintln!("Warning: Failed to add dependency '{}': {}", name, e);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn roots(&self) -> &HashSet<PathBuf> {
        &self.roots
    }
}