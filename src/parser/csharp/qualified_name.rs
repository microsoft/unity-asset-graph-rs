use std::{
    fmt::{Display, Formatter, Result},
    borrow::Borrow,
    hash::Hash,
};
use serde::{Deserialize, Serialize};

pub trait QualifiedName: Debug+Clone+PartialEq+Eq+PartialOrd+Ord+Hash+Serialize+Deserialize {
    fn parts(&self) -> &[impl Borrow<str>];
}

/// A C# qualified name, represented as parts in reverse order (e.g. ["MyClass", "MyNamespace"])
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum QualifiedNameOwned {
    Partial(Vec<String>),
    Full(Vec<String>),

}

impl QualifiedName {
    pub fn global() -> Self {
        Self::Full(vec![])
    }

    pub fn new(parts: Vec<String>) -> Self {
        if parts.is_empty() {
            panic!("QualifiedName must have at least one part");
        }
        Self::Partial(parts)
    }

    pub fn from_iter(parts: impl Iterator<Item = impl Into<String>>) -> Self {
        Self::new(parts.map(|s| s.into()).collect())
    }

    pub fn from_name(name: impl Into<String>, namespace: Self) -> Self {
        match namespace {
            Self::Partial(mut p) => {
                p.insert(0, name.into());
                Self::Partial(p)
            },
            Self::Full(mut p) => {
                p.insert(0, name.into());
                Self::Full(p)
            },
        }
    }

    pub fn resolve(self) -> Self {
        let parts = match self {
            Self::Partial(p) | Self::Full(p) => p,
        };
        Self::Full(parts)
    }

    pub fn concat(narrow: &Self, broad: &Self) -> Self {
        let new = Self::from_iter(narrow.iter().chain(broad.iter()));
        if let Self::Full(_) = broad {
            new.resolve()
        } else {
            new
        }
    }

    /// Whether another name is within this namespace
    pub fn contains(&self, other: &Self) -> bool {
        self.iter().rev().eq(other.iter().rev().take(self.len()))
    }

    /// The containing type or namespace of this name
    pub fn container(&self) -> Self {
        match self {
            Self::Partial(p) => Self::Partial(p.iter().skip(1).cloned().collect()),
            Self::Full(p) => Self::Full(p.iter().skip(1).cloned().collect()),
        }
    }

    /// Produce a less qualified name by removing the given namespace. Returns None if this name is not within the namespace.
    pub fn without_namespace(&self, ns: &Self) -> Option<Self> {
        let mut parts = match self {
            Self::Partial(p) | Self::Full(p) => p.clone(),
        };
        for ns_part in ns.iter().rev() {
            if parts.last() == Some(ns_part) {
                parts.pop();
            } else {
                return None;
            }
        }
        Some(Self::Partial(parts))
    }

    /// Produce a namespace by removing the given local name. Returns None if the local name is not within this namespace.
    pub fn without_local(&self, local: &Self) -> Option<Self> {
        if !self.iter().take(local.len()).eq(local.iter()) {
            None
        } else {
            let parts = self.iter().skip(local.len()).cloned().collect();
            match self {
                Self::Partial(_) => Some(Self::Partial(parts)),
                Self::Full(_) => Some(Self::Full(parts)),
            }
        }
    }

    pub fn local(&self) -> Self {
        Self::Partial(self.iter().take(1).cloned().collect())
    }

    pub fn namespace(&self) -> Self {
        match self {
            Self::Partial(p) => Self::Partial(p.iter().skip(1).cloned().collect()),
            Self::Full(p) => Self::Full(p.iter().skip(1).cloned().collect()),
        }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &String> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Partial(p) | Self::Full(p) => p.len()
        }
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
