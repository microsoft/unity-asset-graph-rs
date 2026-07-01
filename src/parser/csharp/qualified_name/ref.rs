use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt::{Display, Formatter, Result as FResult},
    sync::LazyLock,
};
use tree_sitter::Node;

use super::super::queries::kinds as k;
use super::{
    Error, GENERIC_NAMES, NamePart, QualifiedName, QualifiedNameOwned, QualifiedNamePart, generic_args_count_from_str,
};

#[derive(Debug, Clone, Eq, Ord, Hash)]
pub struct NamePartRef<'a> {
    pub name: &'a str,
    pub generics: usize,
}

impl<'a> NamePartRef<'a> {
    pub fn to_owned(&self) -> NamePart {
        NamePart {
            name: self.name.to_string(),
            generics: self.generics,
        }
    }
}

impl<'a> QualifiedNamePart for NamePartRef<'a> {
    fn name(&self) -> &str {
        self.name
    }
    fn generics(&self) -> usize {
        self.generics
    }
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
            Self {
                name: n,
                generics: generic_args_count_from_str(g),
            }
        } else {
            Self {
                name: value,
                generics: 0,
            }
        }
    }
}

impl<'a> From<&'a Self> for NamePartRef<'a> {
    fn from(value: &'a Self) -> Self {
        value.clone()
    }
}

impl<'a> From<&'a NamePart> for NamePartRef<'a> {
    fn from(value: &'a NamePart) -> Self {
        Self {
            name: value.name(),
            generics: value.generics(),
        }
    }
}

impl<'a, T> PartialEq<T> for NamePartRef<'a>
where
    T: QualifiedNamePart,
{
    fn eq(&self, other: &T) -> bool {
        self.name == other.name() && self.generics == other.generics()
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

impl<'a, T> PartialOrd<T> for NamePartRef<'a>
where
    T: QualifiedNamePart,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        Some(self.name.cmp(other.name()).then(self.generics.cmp(&other.generics())))
    }
}

/// A C# qualified name, represented as parts in order (e.g. ["MyNamespace", "MyClass"])
#[derive(Default, Debug, Clone, Eq, Ord, Hash)]
pub struct QualifiedNameRef<'a> {
    pub parts: Vec<NamePartRef<'a>>,
    pub alias: Option<&'a str>,
}

impl<'a> QualifiedNameRef<'a> {
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

    pub fn try_from<'t, 'b>(node: Node<'t>, buffer: &'b [u8]) -> Result<Self, Error>
    where
        'b: 'a,
    {
        let mut name = Self { ..Default::default() };
        try_from(node, buffer, &mut name)?;
        Ok(name)
    }

    pub fn push(&mut self, part: &'a str) {
        self.parts.push(part.into());
    }

    pub fn pop(&mut self) -> Option<&'a str> {
        self.parts.pop().map(|p| p.name)
    }

    pub fn to_owned(&self) -> QualifiedNameOwned {
        QualifiedNameOwned {
            parts: self.parts.iter().map(|p| p.to_owned()).collect(),
            alias: self.alias.map(|s| s.to_string()),
        }
    }
}

impl<'a> QualifiedName for QualifiedNameRef<'a> {
    type Part = NamePartRef<'a>;
    type Str = &'a str;

    fn global() -> Self {
        Self {
            parts: vec![],
            alias: Some("global"),
        }
    }

    fn parts(&self) -> impl ExactSizeIterator<Item = &Self::Part> {
        self.parts.iter()
    }

    fn alias(&self) -> Option<&Self::Str> {
        self.alias.as_ref()
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

impl<'a, T, P, S> PartialEq<T> for QualifiedNameRef<'a>
where
    T: QualifiedName<Part = P, Str = S>,
    P: PartialEq<NamePartRef<'a>>,
    S: PartialEq<&'a str>,
{
    fn eq(&self, other: &T) -> bool {
        if let Some(a) = self.alias {
            if let Some(o) = other.alias()
                && *o != a
            {
                return false;
            }
        }
        other.parts().eq(self.parts())
    }
}

impl<'a, T, P, S> PartialOrd<T> for QualifiedNameRef<'a>
where
    T: QualifiedName<Part = P, Str = S>,
    P: PartialOrd<NamePartRef<'a>>,
    S: PartialOrd<&'a str>,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        other
            .alias()
            .into_iter()
            .partial_cmp(&self.alias)
            .and(other.parts().partial_cmp(self.parts.iter()))
    }
}

impl<'a> FromIterator<NamePartRef<'a>> for QualifiedNameRef<'a> {
    fn from_iter<T: IntoIterator<Item = NamePartRef<'a>>>(iter: T) -> Self {
        Self {
            parts: iter.into_iter().collect(),
            ..Default::default()
        }
    }
}

impl<'a> FromIterator<&'a str> for QualifiedNameRef<'a> {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        Self {
            parts: iter.into_iter().map(|s| NamePartRef::from(s)).collect(),
            ..Default::default()
        }
    }
}

impl<'a> IntoIterator for QualifiedNameRef<'a> {
    type IntoIter = std::vec::IntoIter<NamePartRef<'a>>;
    type Item = NamePartRef<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.parts.into_iter()
    }
}

impl<'a> From<&[&'a str]> for QualifiedNameRef<'a> {
    fn from(value: &[&'a str]) -> Self {
        Self::from_iter(value.into_iter().map(|s| *s))
    }
}

impl<'a> From<&'a str> for QualifiedNameRef<'a> {
    fn from(value: &'a str) -> Self {
        if let Some((alias, rest)) = value.split_once("::")
            && !alias.contains(|c: char| !c.is_alphanumeric())
        {
            let mut new = Self::from_iter(rest.split('.'));
            new.alias = Some(alias);
            new
        } else {
            Self::from_iter(value.split('.'))
        }
    }
}

impl<'a> From<&'a Self> for QualifiedNameRef<'a> {
    fn from(value: &'a Self) -> Self {
        value.clone()
    }
}

impl<'a> From<&'a QualifiedNameOwned> for QualifiedNameRef<'a> {
    fn from(value: &'a QualifiedNameOwned) -> Self {
        Self {
            parts: value.parts.iter().map(|p| p.into()).collect(),
            alias: value.alias.as_ref().map(|s| s.as_str()),
        }
    }
}

impl<'a> PartialEq<str> for QualifiedNameRef<'a> {
    fn eq(&self, other: &str) -> bool {
        let other = QualifiedNameRef::from(other);
        self == &other
    }
}

impl<'a> Display for QualifiedNameRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        if let Some(alias) = self.alias {
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

static MAE_TERMINALS: LazyLock<HashSet<u16>> =
    LazyLock::new(|| HashSet::from([*k::INVOCATION_EXPR, *k::ELEMENT_ACCESS_EXPR]));

/// Extract a qualified name recursively from the source tree. Note: outputs parts in reverse order.
fn try_from<'t, 'b, 'n>(node: Node<'t>, buffer: &'b [u8], output: &mut QualifiedNameRef<'n>) -> Result<(), Error>
where
    'b: 'n,
{
    match node.kind() {
        "identifier" => {
            let name = node.utf8_text(buffer).map_err(|e| Error::Utf8(e))?;
            output.parts.push(NamePartRef { name, generics: 0 });

            // if this identifier is the start of a member access chain, include the whole chain in the name
            // because member_access_expressions are syntactically identical to qualified_names
            let mut parent = node;
            while let Some(p) = parent.parent()
                && p.kind_id() == *k::MEMBER_ACCESS_EXPR
            {
                // end the chain early when this token is obviously not a type name
                if let Some(a) = p.parent()
                    && MAE_TERMINALS.contains(&a.kind_id())
                {
                    break;
                }
                if let Some(part_node) = p.child_by_field_name("name")
                    && part_node != node
                {
                    try_from(part_node, buffer, output)?;
                    // generic names are not valid namespace names, but are type names so end after this
                    if part_node.kind_id() == *k::GENERIC_NAME {
                        break;
                    }
                }
                parent = p;
            }

            Ok(())
        }
        "generic_name" => {
            // children: identifier, type_argument_list
            let mut name = NamePartRef { name: "", generics: 0 };
            let mut cursor = node.walk();
            for c in node.named_children(&mut cursor) {
                match c.kind() {
                    "identifier" => {
                        name.name = c.utf8_text(buffer).map_err(|e| Error::Utf8(e))?;
                    }
                    "type_argument_list" => {
                        name.generics = c.named_child_count();
                    }
                    _ => {
                        return Err(Error::BadGeneric(
                            node.utf8_text(buffer)
                                .map(|s| s.to_string())
                                .map_err(|e| Error::Utf8(e))?,
                        ));
                    }
                }
            }
            output.parts.push(name);
            Ok(())
        }
        "qualified_name" => {
            let (name, qualifier) = match (node.child_by_field_name("name"), node.child_by_field_name("qualifier")) {
                (Some(n), Some(q)) => (n, q),
                _ => {
                    return Err(Error::BadQualified(
                        node.utf8_text(buffer)
                            .map(|s| s.to_string())
                            .map_err(|e| Error::Utf8(e))?,
                    ));
                }
            };
            try_from(qualifier, buffer, output)?;
            try_from(name, buffer, output)?;
            Ok(())
        }
        "alias_qualified_name" => {
            let (alias, name) = match (node.child_by_field_name("alias"), node.child_by_field_name("name")) {
                (Some(a), Some(n)) => (a, n),
                _ => {
                    return Err(Error::BadQualified(
                        node.utf8_text(buffer)
                            .map(|s| s.to_string())
                            .map_err(|e| Error::Utf8(e))?,
                    ));
                }
            };
            output.alias = Some(alias.utf8_text(buffer).map_err(|e| Error::Utf8(e))?);
            try_from(name, buffer, output)
        }
        _ => Err(Error::BadKind(node.kind().to_string())),
    }
}
