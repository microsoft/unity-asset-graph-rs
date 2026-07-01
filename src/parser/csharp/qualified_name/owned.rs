use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FResult};
use tree_sitter::Node;

use super::{
    Error, GENERIC_NAMES, NamePartRef, QualifiedName, QualifiedNamePart, QualifiedNameRef, generic_args_count_from_str,
};

#[derive(Debug, Clone, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct NamePart {
    pub name: String,
    pub generics: usize,
}

impl QualifiedNamePart for NamePart {
    fn name(&self) -> &str {
        &self.name
    }
    fn generics(&self) -> usize {
        self.generics
    }
}

impl NamePart {
    pub fn as_ref(&self) -> NamePartRef<'_> {
        NamePartRef {
            name: self.name.as_str(),
            generics: self.generics,
        }
    }
}

impl Display for NamePart {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        write!(f, "{}", self.name)?;
        if self.generics > 0 {
            write!(f, "{}", GENERIC_NAMES[self.generics - 1])?;
        }
        Ok(())
    }
}

impl From<&NamePartRef<'_>> for NamePart {
    fn from(value: &NamePartRef<'_>) -> Self {
        Self {
            name: value.name.to_string(),
            generics: value.generics,
        }
    }
}

impl From<&Self> for NamePart {
    fn from(value: &Self) -> Self {
        value.clone()
    }
}

impl From<&str> for NamePart {
    fn from(value: &str) -> Self {
        if let Some(open_index) = value.find('<') {
            let (n, g) = value.split_at(open_index);
            Self {
                name: n.to_string(),
                generics: generic_args_count_from_str(g),
            }
        } else {
            Self {
                name: value.to_string(),
                generics: 0,
            }
        }
    }
}

impl<T> PartialEq<T> for NamePart
where
    T: QualifiedNamePart,
{
    fn eq(&self, other: &T) -> bool {
        &self.name == other.name() && self.generics == other.generics()
    }
}

impl PartialEq<str> for NamePart {
    fn eq(&self, other: &str) -> bool {
        if other.len() < self.name.len() {
            return false;
        }

        let (n, g) = other.split_at(self.name.len());
        n == self.name && generic_args_count_from_str(g) == self.generics
    }
}

impl<T> PartialOrd<T> for NamePart
where
    T: QualifiedNamePart,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        Some(
            self.name
                .as_str()
                .cmp(other.name())
                .then(self.generics.cmp(&other.generics())),
        )
    }
}

/// A C# qualified name, represented as parts in order (e.g. ["MyNamespace", "MyClass"])
#[derive(Default, Debug, Clone, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct QualifiedNameOwned {
    pub parts: Vec<NamePart>,
    pub alias: Option<String>,
}

impl QualifiedNameOwned {
    pub fn as_ref(&self) -> QualifiedNameRef<'_> {
        QualifiedNameRef {
            parts: self.parts.iter().map(|p| p.as_ref()).collect(),
            alias: self.alias.as_ref().map(|s| s.as_str()),
        }
    }

    pub fn try_concat(start: impl Into<Self>, end: impl Into<Self>) -> Result<Self, Error> {
        let start = start.into();
        let end = end.into();
        if let Some(alias) = end.alias {
            return Err(Error::BadAlias(alias.to_string()));
        }

        Ok(Self {
            alias: start.alias,
            parts: start.parts.into_iter().chain(end.parts.into_iter()).collect(),
        })
    }

    pub fn concat(start: impl Into<Self>, end: impl Into<Self>) -> Self {
        let start = start.into();
        let end = end.into();
        if let Some(alias) = end.alias {
            panic!("Trailing name in a concat operation cannot have a namespace alias");
        }

        Self {
            alias: start.alias,
            parts: start.parts.into_iter().chain(end.parts.into_iter()).collect(),
        }
    }
}

impl QualifiedName for QualifiedNameOwned {
    type Part = NamePart;
    type Str = String;

    fn global() -> Self {
        Self {
            parts: vec![],
            alias: Some("global".into()),
        }
    }

    fn alias(&self) -> Option<&Self::Str> {
        self.alias.as_ref()
    }

    fn parts(&self) -> impl ExactSizeIterator<Item = &Self::Part> {
        self.parts.iter()
    }

    fn split_off(&mut self, index: usize) -> Self {
        Self {
            parts: self.parts.split_off(index),
            ..Default::default()
        }
    }

    fn resolve_alias(&mut self, namespace: Self) {
        if self.alias.is_some() {
            self.alias = namespace.alias;
            for p in namespace.parts.into_iter().rev() {
                self.parts.insert(0, p);
            }
        }
    }
}

impl<'a, T, P, S> PartialEq<T> for QualifiedNameOwned
where
    T: QualifiedName<Part = P, Str = S>,
    P: PartialEq<NamePart>,
    S: PartialEq<String>,
{
    fn eq(&self, other: &T) -> bool {
        if let Some(a) = &self.alias {
            if let Some(o) = other.alias()
                && o != a
            {
                return false;
            }
        }
        other.parts().eq(self.parts())
    }
}

impl<T, P, S> PartialOrd<T> for QualifiedNameOwned
where
    T: QualifiedName<Part = P, Str = S>,
    P: PartialOrd<NamePart>,
    S: PartialOrd<String>,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        other
            .alias()
            .into_iter()
            .partial_cmp(&self.alias)
            .and(other.parts().partial_cmp(self.parts.iter()))
    }
}

impl IntoIterator for QualifiedNameOwned {
    type IntoIter = std::vec::IntoIter<NamePart>;
    type Item = NamePart;
    fn into_iter(self) -> Self::IntoIter {
        self.parts.into_iter()
    }
}

impl From<&str> for QualifiedNameOwned {
    fn from(value: &str) -> Self {
        QualifiedNameRef::from(value).to_owned()
    }
}

impl From<String> for QualifiedNameOwned {
    fn from(value: String) -> Self {
        QualifiedNameRef::from(value.as_str()).to_owned()
    }
}

impl From<&Self> for QualifiedNameOwned {
    fn from(value: &Self) -> Self {
        value.clone()
    }
}

impl Display for QualifiedNameOwned {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        if let Some(alias) = &self.alias {
            write!(f, "{alias}::")?;
        }
        let mut iter = self.parts.iter();
        if let Some(p) = iter.next() {
            write!(f, "{}", p)?;
        }
        for part in iter {
            write!(f, ".{}", part)?;
        }
        Ok(())
    }
}

impl PartialEq<str> for QualifiedNameOwned {
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == QualifiedNameRef::from(other)
    }
}
