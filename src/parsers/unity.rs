use std::{
    collections::HashSet,
    io::BufRead,
    path::PathBuf,
};
use regex::Regex;
use uuid::Uuid;
use crate::{
    id::Id,
    parsers::ParseError,
    parsers::localized_text::LocStringParser,
};

pub fn parse(path: &PathBuf) -> Result<HashSet<Id>, ParseError> {
    let id_regex = Regex::new(r"\d{32}").unwrap();

    let mut dependencies = HashSet::new();

    let mut reader = match crate::util::read_file_no_bom(path) {
        Ok(file) => file,
        Err(e) => return Err(ParseError {
            message: format!("Failed to read prefab file: {}", e),
            inner: None,
        }),
    };

    let mut loc_parser = LocStringParser::Start;
    let mut line = String::new();
    while let Ok(bytes) = reader.read_line(&mut line) && bytes > 0 {
        loc_parser = loc_parser.update(&line);
        if let LocStringParser::LocStringKey(id) = loc_parser {
            dependencies.insert(id);
            loc_parser = LocStringParser::Start;
        }

        if let Some(captures) = id_regex.captures(&line)
            && let Some(id_str) = captures.get(0) {
            
            match Uuid::parse_str(id_str.as_str()) {
                Ok(uuid) => dependencies.insert(Id::Guid(uuid)),
                Err(_) => {
                    return Err(ParseError {
                        message: format!("Invalid UUID found in prefab: {}", id_str.as_str()),
                        inner: None,
                    });
                }
            };
        }

        line.clear();
    }

    Ok(dependencies)
}