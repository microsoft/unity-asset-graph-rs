use std::collections::HashMap;

use crate::parser::csharp::qualified_name::{NamePartRef, QualifiedName, QualifiedNameRef};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QualifiedNameSearchTree<'a> {
    names: HashMap<NamePartRef<'a>, Box<QualifiedNameSearchTree<'a>>>,
}

impl<'a> QualifiedNameSearchTree<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Add a fully-qualified name to this search tree. Returns true if newly added, false if it was
    /// already present.
    pub fn insert(&mut self, name: impl Into<QualifiedNameRef<'a>>) -> bool {
        let name = name.into();
        if name.len() == 0 || name.len() == 1 && self.names.contains_key(&name.parts[0]) {
            return false;
        }

        let (root, rest) = name.split(1);
        let root = root.parts.into_iter().next().unwrap();

        if let Some(map) = self.names.get_mut(&root) {
            map.insert(rest)
        } else {
            self.names.insert(root, Box::new(Self::from_iter([rest])));
            true
        }
    }

    /// Remove a fully-qualified name from this search tree. Returns true if removed, false if
    /// absent to begin with.
    pub fn remove(&mut self, name: impl Into<QualifiedNameRef<'a>>) -> bool {
        let name = name.into();
        if self.len() == 0 && name.len() == 0 {
            return true;
        } else if name.len() == 0 {
            return false;
        }

        let (root, rest) = name.split(1);
        let root = root.parts.into_iter().next().unwrap();

        let (rm_deep, rm_local) = if let Some(map) = self.names.get_mut(&root) {
            let deep = map.len() == 0 && rest.len() == 0 || map.remove(rest);
            (deep, map.len() == 0)
        } else {
            (false, false)
        };

        if rm_local {
            self.names.remove(&root);
        }
        rm_deep
    }

    pub fn get<'b>(&self, name: impl Into<QualifiedNameRef<'b>>) -> Option<&Self>
    where
        'b: 'a,
    {
        let name = name.into();
        if name.len() == 0 {
            Some(self)
        } else if let Some(map) = self.names.get(&name.parts[0]) {
            let (_, rest) = name.split(1);
            map.get(rest)
        } else {
            None
        }
    }

    /// Whether the supplied name is within the search tree.
    pub fn contains<'b>(&self, name: impl Into<QualifiedNameRef<'b>>) -> bool {
        let name = name.into();
        if self.len() == 0 && name.len() == 0 {
            return true;
        } else if name.len() == 0 {
            return false;
        }

        let (root, rest) = name.split(1);
        if let Some(map) = self.names.get(&root.parts[0]) {
            map.contains(rest)
        } else {
            false
        }
    }
}

impl<'a, T> FromIterator<T> for QualifiedNameSearchTree<'a>
where
    T: Into<QualifiedNameRef<'a>>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut tree = Self::new();
        for name in iter {
            tree.insert(name);
        }
        tree
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn insert() {
        let mut tree = QualifiedNameSearchTree::new();
        assert!(tree.insert("A.B.C"));
        assert!(tree.insert("A.B.Z"));
        assert!(tree.insert("A.X"));

        assert_eq!(
            &tree,
            &QualifiedNameSearchTree {
                names: HashMap::from_iter([(
                    NamePartRef::from("A"),
                    Box::new(QualifiedNameSearchTree {
                        names: HashMap::from_iter([
                            (
                                NamePartRef::from("B"),
                                Box::new(QualifiedNameSearchTree {
                                    names: HashMap::from_iter([
                                        (NamePartRef::from("C"), Box::new(QualifiedNameSearchTree::new())),
                                        (NamePartRef::from("Z"), Box::new(QualifiedNameSearchTree::new())),
                                    ])
                                })
                            ),
                            (NamePartRef::from("X"), Box::new(QualifiedNameSearchTree::new())),
                        ])
                    })
                ),])
            }
        );
    }

    #[test]
    fn remove() {
        let mut tree = QualifiedNameSearchTree::from_iter(["A.B.C", "A.B.Z", "A.X"]);
        assert!(!tree.remove("Z"));
        assert!(!tree.remove("A.Z"));
        assert!(!tree.remove("A"));

        assert!(tree.remove("A.B.C"));
        assert_eq!(
            &tree,
            &QualifiedNameSearchTree {
                names: HashMap::from_iter([(
                    NamePartRef::from("A"),
                    Box::new(QualifiedNameSearchTree {
                        names: HashMap::from_iter([
                            (
                                NamePartRef::from("B"),
                                Box::new(QualifiedNameSearchTree {
                                    names: HashMap::from_iter([(
                                        NamePartRef::from("Z"),
                                        Box::new(QualifiedNameSearchTree::new())
                                    ),])
                                })
                            ),
                            (NamePartRef::from("X"), Box::new(QualifiedNameSearchTree::new())),
                        ])
                    })
                ),])
            }
        );

        assert!(tree.remove("A.B.Z"));
        assert_eq!(
            &tree,
            &QualifiedNameSearchTree {
                names: HashMap::from_iter([(
                    NamePartRef::from("A"),
                    Box::new(QualifiedNameSearchTree {
                        names: HashMap::from_iter(
                            [(NamePartRef::from("X"), Box::new(QualifiedNameSearchTree::new())),]
                        )
                    })
                ),])
            }
        );
    }

    #[test]
    fn contains() {
        let tree = QualifiedNameSearchTree::from_iter(["A.B.C", "A.B.Z", "A.X"]);
        assert!(!tree.contains("A.B"));
        assert!(tree.contains("A.B.C"));
    }
}
