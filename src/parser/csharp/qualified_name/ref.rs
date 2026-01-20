use serde::{Deserialize, Serialize};
use tree_sitter::{Node};
use std::fmt::{Display, Formatter, Result as FResult};
use crate::parser::csharp::qualified_name::generic_args_count_from_str;

use super::{Error, GENERIC_NAMES};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamePartRef<'a> {
    name: &'a str,
    generics: usize,
}

impl<'a> Display for NamePartRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        write!(f, "{}", self.name)?;
        if self.generics > 0 {
            write!(f, "{}", GENERIC_NAMES[self.generics - 1])?;
        }
        Ok(())
    }
}

impl<'a> From<&'a str> for NamePartRef<'a> {
    fn from(value: &'a str) -> Self {
        if let Some(open_index) = value.find('<') {
            let (n, g) = value.split_at(open_index);
            Self { name: n, generics: generic_args_count_from_str(g) }
        } else {
            Self { name: value, generics: 0 }
        }
    }
}

impl<'a> PartialEq<str> for NamePartRef<'a> {
    fn eq(&self, other: &str) -> bool {
        if other.len() < self.name.len() {
            return false;
        }

        let (n, g) = other.split_at(self.name.len());
        n == self.name && generic_args_count_from_str(g) == self.generics
    }
}

/// A C# qualified name, represented as parts in order (e.g. ["MyNamespace", "MyClass"])
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QualifiedNameRef<'a>(Vec<NamePartRef<'a>>);

impl<'a> QualifiedNameRef<'a> {
    pub fn concat(start: Self, end: Self) -> Self {
        Self::from_iter(start.into_iter().chain(end.into_iter()))
    }

    pub fn try_from<'t, 'b>(node: Node<'t>, buffer: &'b [u8]) -> Result<Self, Error<'b>>
        where 'b: 'a {
        let mut name = Self(vec![]);
        try_from(node, buffer, &mut name)?;
        Ok(name)
    }

    pub fn starts_with(&self, other: &QualifiedNameRef) -> bool {
        self.0[..other.0.len()] == other.0[..]
    }

    pub fn ends_with(&self, other: &QualifiedNameRef) -> bool {
        self.0[self.0.len() - other.0.len()..] == other.0[..]
    }

    pub fn trim_start(&mut self, other: &QualifiedNameRef) {
        if self.starts_with(other) {
            self.0 = self.0[other.0.len()..].to_vec();
        }
    }

    pub fn trim_end(&mut self, other: &QualifiedNameRef) {
        if self.ends_with(other) {
            let new_len = self.0.len() - other.0.len();
            self.0.truncate(new_len);
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, part: &'a str) {
        self.0.push(part.into());
    }

    pub fn pop(&mut self) -> Option<&'a str> {
        self.0.pop().map(|p| p.name)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, NamePartRef<'a>> {
        self.0.iter()
    }
}

impl<'a> FromIterator<NamePartRef<'a>> for QualifiedNameRef<'a> {
    fn from_iter<T: IntoIterator<Item = NamePartRef<'a>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'a> FromIterator<&'a str> for QualifiedNameRef<'a> {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        Self(iter.into_iter().map(|s| NamePartRef::from(s)).collect())
    }
}

impl<'a> IntoIterator for QualifiedNameRef<'a> {
    type IntoIter = std::vec::IntoIter<NamePartRef<'a>>;
    type Item = NamePartRef<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> From<&[&'a str]> for QualifiedNameRef<'a> {
    fn from(value: &[&'a str]) -> Self {
        Self::from_iter(value.into_iter().map(|s| *s))
    }
}

impl<'a> From<&'a str> for QualifiedNameRef<'a> {
    fn from(value: &'a str) -> Self {
        Self::from_iter(value.split('.'))
    }
}

impl<'a> Display for QualifiedNameRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        let mut iter = self.0.iter();
        if let Some(p) = iter.next() {
            write!(f, "{}", p)?;
        }
        for part in iter {
            write!(f, ".{}", part)?;
        }
        Ok(())
    }
}

impl<'a> PartialEq<str> for QualifiedNameRef<'a> {
    fn eq(&self, other: &str) -> bool {
        self.iter().eq(other.split('.'))
    }
}

/// Extract a qualified name recursively from the source tree. Note: outputs parts in reverse order.
fn try_from<'t, 'b, 'n>(node: Node<'t>, buffer: &'b [u8], output: &mut QualifiedNameRef<'n>) -> Result<(), Error<'b>>
where 'b: 'n {
    match node.kind() {
        "identifier" => {
            let name = node.utf8_text(buffer)
                .map_err(|e| Error::Utf8(e))?;
            output.0.push(NamePartRef { name, generics: 0 });
            Ok(())
        },
        "generic_name" => {
            // children: identifier, type_argument_list
            let mut name = NamePartRef { name: "", generics: 0 };
            let mut cursor = node.walk();
            for c in node.named_children(&mut cursor) {
                match c.kind() {
                    "identifier" => {
                        name.name = c.utf8_text(buffer).map_err(|e| Error::Utf8(e))?;
                    },
                    "type_argument_list" => {
                        name.generics = c.named_child_count();
                    },
                    _ => return Err(Error::BadGeneric(node.utf8_text(buffer).map_err(|e| Error::Utf8(e))?)),
                }
            }
            output.0.push(name);
            Ok(())
        },
        "qualified_name" => {
            let (name, qualifier) = match (node.child_by_field_name("name"), node.child_by_field_name("qualifier")) {
                (Some(n), Some(q)) => (n, q),
                _ => return Err(Error::BadQualified(node.utf8_text(buffer).map_err(|e| Error::Utf8(e))?)),
            };
            try_from(qualifier, buffer, output)?;
            try_from(name, buffer, output)?;
            Ok(())
        },
        _ => Err(Error::BadKind(node.kind())),
    }
}