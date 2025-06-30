use std::{
    fs::File,
    io::{self, BufReader, BufRead},
    path::PathBuf,
};

pub fn read_file_no_bom(path: &PathBuf) -> Result<BufReader<File>, io::Error> {
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