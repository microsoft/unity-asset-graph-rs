mod owned;
mod r#ref;
pub use owned::*;
pub use r#ref::*;

use std::{
    fmt::{Display, Formatter, Result as FResult},
    borrow::Borrow,
};

const GENERIC_NAMES: [&str; 7] = [
    "<T>",
    "<T,U>",
    "<T,U,V>",
    "<T,U,V,W>",
    "<T,U,V,W,X>",
    "<T,U,V,W,X,Y>",
    "<T,U,V,W,X,Y,Z>"
];

#[derive(Debug)]
pub enum Error {
    BadKind(String),
    Utf8(std::str::Utf8Error),
    BadGeneric(String),
    BadQualified(String),
    BadAlias(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        match self {
            Self::BadKind(k) => write!(f, "Bad kind: {k}"),
            Self::Utf8(e) => write!(f, "Bad UTF8 text: {e}"),
            Self::BadGeneric(s) => write!(f, "Failed to parse generic type name '{s}'"),
            Self::BadQualified(s) => write!(f, "Failed to parse qualified type name '{s}'"),
            Self::BadAlias(s) => write!(f, "Failed to handle name alias '{s}'"),
        }
    }
}

impl std::error::Error for Error {}

pub trait QualifiedNamePart: Clone + PartialEq + Eq + PartialOrd + Ord + std::hash::Hash + Display {
    fn name(&self) -> &str;
    fn generics(&self) -> usize;
}

pub trait QualifiedName: Clone + PartialEq + Eq + PartialOrd + Ord + std::hash::Hash + Sized + Display {
    type Part: QualifiedNamePart;
    type Str: Borrow<str>;

    fn parts(&self) -> impl ExactSizeIterator<Item=&Self::Part>;
    fn alias(&self) -> Option<&Self::Str>;

    /// Splits the name into two at the given index. [0, index) is left here, [index, len) is in the returned name
    fn split_off(&mut self, index: usize) -> Self;

    fn resolve_alias(&mut self, namespace: Self);

    fn len(&self) -> usize {
        self.parts().len()
    }

    fn ends_with<P, S>(&self, other: &impl QualifiedName<Part=P, Str=S>) -> bool
    where Self::Part: PartialEq<P>, Self::Str: PartialEq<S> {
        if let Some(oa) = other.alias() {
            if let Some(sa) = self.alias() {
                sa == oa && self.parts().eq(other.parts())
            } else {
                false
            }
        } else if self.parts().len() >= other.parts().len() {
            self.parts().skip(self.len() - other.len()).eq(other.parts())
        } else {
            false
        }
    }

    fn trim_end<P, S>(&mut self, other: &impl QualifiedName<Part=P, Str=S>)
    where Self::Part: PartialEq<P>, Self::Str: PartialEq<S> {
        if self.ends_with(other) {
            self.split_off(self.len() - other.len());
        }
    }
}

fn generic_args_count_from_str(text: &str) -> usize {
    let mut count = 0usize;
    let mut depth = 0usize;
    for c in text.chars() {
        if c == '<' {
            depth += 1;
            if depth == 1 {
                count += 1;
            }
        } else if c == '>' && depth > 0 {
            depth -= 1;
        } else if c == ',' && depth == 1 {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn generic_args_count() {
        assert_eq!(generic_args_count_from_str("Test"), 0);
        assert_eq!(generic_args_count_from_str("Test<T>"), 1);
        assert_eq!(generic_args_count_from_str("Test<T,U>"), 2);
        assert_eq!(generic_args_count_from_str("Test<T<U,V>>"), 1);
    }
}
