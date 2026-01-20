mod owned;
mod r#ref;
pub use owned::*;
pub use r#ref::*;

use std::{
    fmt::{Display, Formatter, Result as FResult},
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
pub enum Error<'a> {
    BadKind(&'a str),
    Utf8(std::str::Utf8Error),
    BadGeneric(&'a str),
    BadQualified(&'a str),
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        match self {
            Self::BadKind(k) => write!(f, "Bad kind: {k}"),
            Self::Utf8(e) => write!(f, "Bad UTF8 text: {e}"),
            Self::BadGeneric(s) => write!(f, "Failed to parse generic type name '{s}'"),
            Self::BadQualified(s) => write!(f, "Failed to parse qualified type name '{s}'"),
        }
    }
}

impl<'a> std::error::Error for Error<'a> {}

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