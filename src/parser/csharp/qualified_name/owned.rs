use serde::{Deserialize, Serialize};
use tree_sitter::{Node};
use std::fmt::{Display, Formatter, Result as FResult};
use super::{Error, GENERIC_NAMES};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct NamePart {
    name: String,
    generics: usize,
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

/// A C# qualified name, represented as parts in order (e.g. ["MyNamespace", "MyClass"])
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QualifiedName(pub Vec<String>);

impl QualifiedName {
    pub fn new(parts: Vec<String>) -> Self {
        if parts.is_empty() {
            panic!("QualifiedName must have at least one part");
        }
        Self(parts)
    }

    pub fn concat(start: Self, end: Self) -> Self {
        Self::from_iter(start.into_iter().chain(end.into_iter()))
    }

    pub fn try_from(node: Node<'_>, buffer: &[u8]) -> Result<Self, Error> {
        let mut name = Self(vec![]);
        try_from(node, buffer, &mut name)?;
        name.0.reverse();
        Ok(name)
    }

    pub fn starts_with(&self, other: &QualifiedName) -> bool {
        self.0[..other.0.len()] == other.0[..]
    }

    pub fn ends_with(&self, other: &QualifiedName) -> bool {
        self.0[self.0.len() - other.0.len()..] == other.0[..]
    }

    pub fn trim_start(&mut self, other: &QualifiedName) {
        if self.starts_with(other) {
            self.0 = self.0[other.0.len()..].to_vec();
        }
    }

    pub fn trim_end(&mut self, other: &QualifiedName) {
        if self.ends_with(other) {
            let new_len = self.0.len() - other.0.len();
            self.0.truncate(new_len);
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, part: String) {
        self.0.push(part);
    }

    pub fn pop(&mut self) -> Option<String> {
        self.0.pop()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.0.iter()
    }
}

impl<'a> FromIterator<&'a str> for QualifiedName {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let parts: Vec<String> = iter.into_iter().map(|s| s.to_string()).collect();
        QualifiedName::new(parts)
    }
}

impl FromIterator<String> for QualifiedName {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        QualifiedName::new(iter.into_iter().collect())
    }
}

impl IntoIterator for QualifiedName {
    type IntoIter = std::vec::IntoIter<String>;
    type Item = String;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<String>> for QualifiedName {
    fn from(value: Vec<String>) -> Self {
        Self::new(value)
    }
}

impl From<&[&str]> for QualifiedName {
    fn from(value: &[&str]) -> Self {
        Self::from_iter(value.iter().cloned())
    }
}

impl From<&str> for QualifiedName {
    fn from(value: &str) -> Self {
        Self::from_iter(value.split('.'))
    }
}

// todo: split in place instead of cloning slices
impl From<String> for QualifiedName {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl Display for QualifiedName {
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

impl PartialEq<str> for QualifiedName {
    fn eq(&self, other: &str) -> bool {
        other.split('.').eq(self.0.iter().map(String::as_str))
    }
}

/// Extract a qualified name recursively from the source tree. Note: outputs parts in reverse order.
fn try_from(node: Node<'_>, buffer: &[u8], output: &mut QualifiedName) -> Result<(), Error> {
    match node.kind() {
        "identifier" => {
            let name = node.utf8_text(buffer)
                .map_err(|e| Error::Utf8(e))?
                .to_string();
            output.0.push(name);
            Ok(())
        },
        "generic_name" => {
            // children: identifier, type_argument_list
            let mut name = String::new();
            let mut cursor = node.walk();
            for c in node.named_children(&mut cursor) {
                match c.kind() {
                    "identifier" => {
                        let id = c.utf8_text(buffer).map_err(|e| Error::Utf8(e))?;
                        name.insert_str(0, id);
                    },
                    "type_argument_list" => {
                        name.push_str(GENERIC_NAMES[c.named_child_count() - 1]);
                    },
                    _ => {
                        return Err(
                            Error::BadGeneric(
                                node.utf8_text(buffer)
                                    .map(|s| s.to_string())
                                    .map_err(|e| Error::Utf8(e))?
                            )
                        );
                    },
                }
            }
            output.push(name);
            Ok(())
        },
        "qualified_name" => {
            let (name, qualifier) = match (node.child_by_field_name("name"), node.child_by_field_name("qualifier")) {
                (Some(n), Some(q)) => (n, q),
                _ => {
                    return Err(Error::BadQualified(
                        node.utf8_text(buffer)
                            .map(|s| s.to_string())
                            .map_err(|e| Error::Utf8(e))?
                    ));
                },
            };
            try_from(name, buffer, output)?;
            try_from(qualifier, buffer, output)?;
            Ok(())
        },
        _ => Err(Error::BadKind(node.kind().to_string())),
    }
}
