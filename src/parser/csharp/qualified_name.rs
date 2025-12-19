use std::fmt::{Display, Formatter, Result};
use serde::{Deserialize, Serialize};

/// A C# qualified name, represented as parts in reverse order (e.g. ["MyClass", "MyNamespace"])
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QualifiedName(Vec<String>);

impl QualifiedName {
    pub fn global() -> Self {
        Self(vec![])
    }

    pub fn is_global(&self) -> bool {
        self.0.is_empty()
    }

    pub fn new(parts: Vec<String>) -> Self {
        if parts.is_empty() {
            panic!("QualifiedName must have at least one part");
        }
        Self(parts)
    }

    pub fn from_parts(parts: impl Iterator<Item = impl Into<String>>) -> Self {
        Self::new(parts.map(|s| s.into()).collect())
    }

    pub fn from_name(name: impl Into<String>, mut namespace: Self) -> Self {
        namespace.0.insert(0, name.into());
        Self(namespace.0)
    }

    pub fn concat(narrow: &Self, broad: &Self) -> Self {
        Self::from_parts(narrow.iter().chain(broad.iter()))
    }

    /// Whether another name is within this namespace
    pub fn contains(&self, other: &Self) -> bool {
        self.iter().rev().eq(other.iter().rev().take(self.0.len()))
    }

    /// The containing type or namespace of this name
    pub fn container(&self) -> Self {
        if self.0.len() <= 1 {
            Self::global()
        } else {
            Self::new(self.0[1..].to_vec())
        }
    }

    /// Produce a less qualified name by removing the given namespace. Returns None if this name is not within the namespace.
    pub fn without_namespace(&self, ns: &Self) -> Option<Self> {
        let mut parts = self.0.clone();
        for ns_part in ns.iter().rev() {
            if parts.last() == Some(ns_part) {
                parts.pop();
            } else {
                return None;
            }
        }
        Some(Self::new(parts))
    }

    /// Produce a namespace by removing the given local name. Returns None if the local name is not within this namespace.
    pub fn without_local(&self, local: &Self) -> Option<Self> {
        let mut parts = self.0.clone();
        for local_part in local.iter() {
            if parts.first() == Some(local_part) {
                parts.remove(0);
            } else {
                return None;
            }
        }
        if parts.is_empty() {
            Some(Self::global().clone())
        } else {
            Some(Self::new(parts))
        }
    }

    pub fn local(&self) -> Self {
        Self::new(vec![self.0[0].clone()])
    }

    pub fn namespace(&self) -> Self {
        Self::new(self.0[1..].to_vec())
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &String> {
        self.0.iter()
    }
}

impl From<Vec<String>> for QualifiedName {
    fn from(value: Vec<String>) -> Self {
        Self::new(value)
    }
}

impl From<&[&str]> for QualifiedName {
    fn from(value: &[&str]) -> Self {
        Self::from_parts(value.iter().cloned())
    }
}

impl From<&str> for QualifiedName {
    fn from(value: &str) -> Self {
        Self::from_parts(value.split('.').rev())
    }
}

// todo: split in place instead of cloning slices
impl From<String> for QualifiedName {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl Display for QualifiedName {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut first = true;
        for part in self.0.iter().rev() {
            if first {
                write!(f, "{}", part)?;
                first = false;
            } else {
                write!(f, ".{}", part)?;
            }
        }
        Ok(())
    }
}

impl PartialEq<&str> for QualifiedName {
    fn eq(&self, other: &&str) -> bool {
        other.split('.').rev().eq(self.iter())
    }
}
