use std::{
    fmt::{Display, Formatter, Result},
    borrow::Borrow,
    hash::Hash,
};
use serde::{Deserialize, Serialize};

pub trait QualifiedName: Eq + Ord + Hash {
    fn parts(&self) -> &[impl Borrow<str>];
    fn is_fully_qualified(&self) -> bool;
}

/// A C# qualified name, represented as parts in reverse order (e.g. ["MyClass", "MyNamespace"])
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FullyQualifiedName {
    parts: Vec<String>,
}

impl FullyQualifiedName {
    pub fn new(parts: Vec<String>) -> Self {
        Self { parts }
    }
}

impl FromIterator<String> for FullyQualifiedName {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self { parts: iter.into_iter().collect() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PartiallyQualifiedName {
    parts: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct FullyQualifiedNameRef<'a> {
    parts: Vec<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct PartiallyQualifiedNameRef<'a> {
    parts: Vec<&'a str>,
}

impl QualifiedName for FullyQualifiedName {
    fn is_fully_qualified(&self) -> bool {
        true
    }
    fn parts(&self) -> &[impl Borrow<str>] {
        &self.parts
    }
}

impl<'a> QualifiedName for FullyQualifiedNameRef<'a> {
    fn is_fully_qualified(&self) -> bool {
        true
    }
    fn parts(&self) -> &[impl Borrow<str>] {
        &self.parts
    }
}

impl QualifiedName for PartiallyQualifiedName {
    fn is_fully_qualified(&self) -> bool {
        false
    }
    fn parts(&self) -> &[impl Borrow<str>] {
        &self.parts
    }
}

impl<'a> QualifiedName for PartiallyQualifiedNameRef<'a> {
    fn is_fully_qualified(&self) -> bool {
        false
    }
    fn parts(&self) -> &[impl Borrow<str>] {
        &self.parts
    }
}

impl<'a> From<&'a FullyQualifiedName> for FullyQualifiedNameRef<'a> {
    fn from(value: &'a FullyQualifiedName) -> Self {
        Self {
            parts: value.parts.iter().map(|s| s.as_str()).collect(),
        }
    }
}

impl<'a> Into<FullyQualifiedName> for FullyQualifiedNameRef<'a> {
    fn into(self) -> FullyQualifiedName {
        FullyQualifiedName { parts: self.parts.iter().map(|s| String::from(*s)).collect() }
    }
}

impl<'a> From<&'a PartiallyQualifiedName> for PartiallyQualifiedNameRef<'a> {
    fn from(value: &'a PartiallyQualifiedName) -> Self {
        Self {
            parts: value.parts.iter().map(|s| s.as_str()).collect(),
        }
    }
}

impl<'a> Into<PartiallyQualifiedName> for PartiallyQualifiedNameRef<'a> {
    fn into(self) -> PartiallyQualifiedName {
        PartiallyQualifiedName { parts: self.parts.iter().map(|s| String::from(*s)).collect() }
    }
}

impl QualifiedName {
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
