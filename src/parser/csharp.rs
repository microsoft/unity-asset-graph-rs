use std::{
    path::PathBuf,
    io::BufRead,
    sync::LazyLock,
};
use regex::Regex;
use crate::{
    asset::Asset,
    id::Id,
    parser::ParseError,
};

static LOC_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"LocStringCache.Get\("([^"]+)"\)"#).expect("Failed to compile locString regex")
});

pub fn parse_csharp(asset: &mut Asset, relative_to: Option<&PathBuf>) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(&asset.path),
        None => &asset.path,
    };

    let mut reader = match crate::util::read_file_no_bom(path) {
        Ok(file) => file,
        Err(e) => return Err(ParseError {
            message: format!("Failed to read prefab file: {}", e),
            inner: None,
        }),
    };

    parse_csharp_reader(&mut reader, asset)
}

fn parse_csharp_reader(reader: &mut dyn BufRead, asset: &mut Asset) -> Result<Vec<Asset>, ParseError> {
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => return Err(ParseError {
                message: format!("Failed to read line: {}", e),
                inner: None,
            }),
        };

        if let Some(captures) = LOC_REGEX.captures(&line)
            && let Some(loc_key) = captures.get(1)
        {
            asset.dependencies.insert(Id::Loc(loc_key.as_str().to_string()));
        }
    }

    Ok(vec![])
}