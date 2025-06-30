use std::{
    collections::{HashMap, HashSet},
    path::{PathBuf},
    fs,
};
use crate::{parsers::{
    manifest_json::ManifestJson,
    package_json::PackageJson,
}, util::read_file_no_bom};
use super::{Database, DatabaseError};

impl Database {
    pub fn add_root_str(&mut self, path: &str) -> Result<(), DatabaseError> {
        let abs_root = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(e) => return Err(DatabaseError {
                message: format!("failed to canonicalize path '{path}'"),
                inner: Some(Box::new(e)),
            }),
        };
        self.add_root(abs_root, &mut HashSet::new())
    }

    fn add_root(
        &mut self,
        path: PathBuf,
        unresolved: &mut HashSet<String>
    ) -> Result<(), DatabaseError> {
        // println!("{}", path.display());
        self.roots.insert(path.clone());

        // check for a manifest.json file
        let manifest_path = path.join("Packages").join("manifest.json");
        if manifest_path.exists() {
            let reader = match read_file_no_bom(&manifest_path) {
                Ok(r) => r,
                Err(e) => return Err(DatabaseError {
                    message: format!("failed to read package file '{}'", manifest_path.display()),
                    inner: Some(Box::new(e)),
                }),
            };
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

                    if self.roots.contains(&dep_abs_path) {
                        continue; // Already added
                    }
                    
                    if dep_abs_path.exists() {
                        self.add_root(dep_abs_path, unresolved)?;
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
            let reader = match read_file_no_bom(&package_path) {
                Ok(r) => r,
                Err(e) => return Err(DatabaseError {
                    message: format!("failed to read package file '{}'", package_path.display()),
                    inner: Some(Box::new(e)),
                }),
            };
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

                    if self.roots.contains(&dep_abs_path) {
                        continue; // Already added
                    }

                    if dep_abs_path.exists() {
                        self.add_root(dep_abs_path, unresolved)?;
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
                    if let Err(e) = self.add_root(dep_path, unresolved) {
                        eprintln!("Warning: Failed to add dependency '{}': {}", name, e);
                    }
                }
            }
        }

        Ok(())
    }
}