use std::{
    collections::{HashMap, HashSet},
    path::{PathBuf},
    fs,
};
use crate::{
    parser::{
        manifest_json::ManifestJson,
        package_json::PackageJson,
    },
    util::read_file_no_bom
};
use super::{Database, DatabaseError};

impl Database {
    pub fn add_root_str(&mut self, path: &str) -> Result<(), DatabaseError> {
        let abs_root = match fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => return Err(DatabaseError {
                message: format!("failed to canonicalize path '{path}'"),
                inner: None,
            }),
        };
        self.add_root(abs_root, &mut HashSet::new())
    }

    /// Adds a root directory to the database, resolving dependencies recursively.
    /// If the directory contains a `manifest.json` or `package.json`, it will read dependencies and add them as well.
    /// If a dependency is a relative path (starting with `file:`), it will resolve it relative to the root directory.
    /// If a dependency is not found, it will be added to the `unresolved` set.
    /// # Arguments
    /// * `path` - The absolute path to the root directory to add.
    fn add_root(
        &mut self,
        path: PathBuf,
        unresolved: &mut HashSet<String>
    ) -> Result<(), DatabaseError> {
        let assets_dir = path.join("Assets");
        let manifest_path = path.join("Packages").join("manifest.json");
        if assets_dir.exists() && manifest_path.exists() {
            self.roots.insert(self.resolve_rel_path(&assets_dir)?);
        }
        else {
            self.roots.insert(self.resolve_rel_path(&path)?);
        }

        // check for a manifest.json file
        if manifest_path.exists() {
            let reader = match read_file_no_bom(&manifest_path) {
                Ok(r) => r,
                Err(_) => return Err(DatabaseError {
                    message: format!("failed to read package file '{}'", manifest_path.display()),
                    inner: None,
                }),
            };
            let manifest: ManifestJson = match serde_json::from_reader(reader) {
                Ok(m) => m,
                Err(_) => return Err(DatabaseError {
                    message: format!("failed to parse manifest file '{}'", manifest_path.display()),
                    inner: None,
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
                Err(_) => return Err(DatabaseError {
                    message: format!("failed to read package file '{}'", package_path.display()),
                    inner: None,
                }),
            };
            let package: PackageJson = match serde_json::from_reader(reader) {
                Ok(p) => p,
                Err(_) => return Err(DatabaseError {
                    message: format!("failed to parse package file '{}'", package_path.display()),
                    inner: None,
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
                Err(_) => return Err(DatabaseError {
                    message: format!("failed to read directory '{}'", lib_path.display()),
                    inner: None,
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

                if name.starts_with("com.unity.") {
                    continue;
                }
                else if dep_path.is_dir() && unresolved.contains(name) {
                    unresolved.remove(name);
                    if let Err(e) = self.add_root(dep_path, unresolved) {
                        eprintln!("Warning: Failed to add dependency '{}': {}", name, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn resolve_rel_path(&self, path: &PathBuf) -> Result<PathBuf, DatabaseError> {
        if let Some(root) = &self.relative_to && let Ok(path) = path.strip_prefix(root) {
            Ok(PathBuf::from(path))
        }
        else if path.is_absolute() {
            Ok(path.clone())
        }
        else {
            Err(DatabaseError {
                message: format!("Path '{}' is not absolute and no relative root is set.", path.display()),
                inner: None,
            })
        }
    }
}