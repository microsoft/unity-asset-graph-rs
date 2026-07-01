use super::{Database, DatabaseError};
use crate::{
    parser::{ParseError, manifest_json::ManifestJson, package_json::PackageJson},
    util::read_file_no_bom,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

impl Database {
    /// Adds a root directory to the database, resolving dependencies recursively.
    /// If the directory contains a `manifest.json` or `package.json`, it will read dependencies and add them as well.
    /// If a dependency is a relative path (starting with `file:`), it will resolve it relative to the root directory.
    /// If a dependency is not found, it will be added to the `unresolved` set.
    /// # Arguments
    /// * `path` - The absolute path to the root directory to add.
    pub fn add_root(&mut self, path: &Path, unresolved: &mut HashSet<String>) -> Result<(), DatabaseError> {
        let assets_dir = path.join("Assets");
        let manifest_path = path.join("Packages").join("manifest.json");
        if assets_dir.exists() && manifest_path.exists() {
            self.roots.insert(self.resolve_rel_path(&assets_dir)?);
        } else {
            self.roots.insert(self.resolve_rel_path(&path)?);
        }

        // check for a manifest.json file
        if manifest_path.exists() {
            let reader = match read_file_no_bom(&manifest_path) {
                Ok(r) => r,
                Err(e) => return Err(DatabaseError::BadPath {
                    path: Some(manifest_path),
                    inner: Some(e),
                }),
            };
            let manifest: ManifestJson = match serde_json::from_reader(reader) {
                Ok(m) => m,
                Err(_) => {
                    return Err(DatabaseError::parse(manifest_path, "Failed to parse manifest file"));
                }
            };

            for (name, version) in manifest.dependencies {
                if version.starts_with("file:") {
                    let dep_path = version.trim_start_matches("file:");
                    let dep_abs_path = path.join("Packages").join(dep_path.trim());

                    if self.roots.contains(&dep_abs_path) {
                        continue; // Already added
                    }

                    if dep_abs_path.exists() {
                        self.add_root(&dep_abs_path, unresolved)?;
                    } else {
                        eprintln!("Warning: Dependency path '{}' does not exist.", dep_abs_path.display());
                    }
                } else {
                    unresolved.insert(name);
                }
            }
        }

        // check for a package.json file
        let package_path = path.join("package.json");
        if package_path.exists() {
            let reader = match read_file_no_bom(&package_path) {
                Ok(r) => r,
                Err(_) => {
                    return Err(DatabaseError::parse(package_path, "Failed to read package file"));
                }
            };
            let package: PackageJson = match serde_json::from_reader(reader) {
                Ok(p) => p,
                Err(_) => {
                    return Err(DatabaseError::parse(package_path, "Failed to parse package file"));
                }
            };

            for (name, version) in package.dependencies.unwrap_or(HashMap::new()) {
                if version.starts_with("file:") {
                    let dep_path = version.trim_start_matches("file:");
                    let dep_abs_path = path.join(dep_path.trim());

                    if self.roots.contains(&dep_abs_path) {
                        continue; // Already added
                    }

                    if dep_abs_path.exists() {
                        self.add_root(&dep_abs_path, unresolved)?;
                    } else {
                        eprintln!("Warning: Dependency path '{}' does not exist.", dep_abs_path.display());
                    }
                } else {
                    unresolved.insert(name);
                }
            }
        }

        // check for a Library/PackageCache directory
        let lib_path = path.join("Library").join("PackageCache");
        if lib_path.exists() {
            let dir = match fs::read_dir(&lib_path) {
                Ok(d) => d,
                Err(e) => return Err(DatabaseError::BadPath {
                    path: Some(lib_path),
                    inner: Some(e),
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
                } else if dep_path.is_dir() && unresolved.contains(name) {
                    unresolved.remove(name);
                    if let Err(e) = self.add_root(&dep_path, unresolved) {
                        eprintln!("Warning: Failed to add dependency '{}': {}", name, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn resolve_rel_path(&self, path: &Path) -> Result<PathBuf, DatabaseError> {
        match path.strip_prefix(&self.relative_to) {
            Ok(path) => Ok(path.to_path_buf()),
            Err(_) if path.is_absolute() => Ok(path.to_path_buf()),
            Err(e) => {
                eprintln!("{path} strips {rel}: {e}", path = path.display(), rel = self.relative_to.display());
                Err(DatabaseError::BadPath {
                   path: Some(path.to_path_buf()),
                    inner: None,
                })
            },
        }
    }
}
