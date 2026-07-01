use crate::{Id, parser::ParseError};
use regex::Regex;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
    sync::LazyLock,
};
use uuid::Uuid;

static META_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^guid: ([0-9a-f]{32})$").expect("Failed to compile meta id regex"));

pub fn read_file_no_bom(path: &Path) -> Result<BufReader<File>, io::Error> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(e),
    };

    let mut reader = BufReader::new(file);

    // consume any BOM at the start of the file
    if let Some(bom) = reader.fill_buf().ok().and_then(|buf| {
        if buf.starts_with(b"\xEF\xBB\xBF") {
            Some(3) // UTF-8 BOM length
        } else {
            None
        }
    }) {
        reader.consume(bom);
    }

    Ok(reader)
}

pub fn get_id_of_asset(path: &Path) -> Result<Id, ParseError> {
    // read the meta file
    let meta_path = path.with_extension(
        path.extension()
            .and_then(|os| os.to_str())
            .map(|e| format!("{e}.meta"))
            .unwrap_or("meta".into()),
    );
    //println!("{}", meta_path.display());

    let meta_reader = read_file_no_bom(&meta_path)
        .map_err(|e| ParseError::new(&meta_path, format!("Failed to read meta file: {e}")))?;

    for line in meta_reader.lines() {
        if let Ok(line) = line
            && let Some(captures) = META_REGEX.captures(&line)
            && let Some(m) = captures.get(1)
            && let Ok(uuid) = Uuid::parse_str(m.as_str())
        {
            // Extract the GUID from the meta file
            return Ok(Id::Guid(uuid));
        }
    }
    Err(ParseError::new(&path, "No uuid found in meta file"))
}
