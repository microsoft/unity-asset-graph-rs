use serde::{Serialize, Deserialize};
use std::{fs::File, path::Path, error::Error, io::Write};
use crate::database::Database;

const MAGIC_BYTE: u8 = 0xae;
const SER_VERSION: u8 = 1;

#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Magic;
impl TryFrom<u8> for Magic {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == MAGIC_BYTE {
            Ok(Magic)
        } else {
            Err("Invalid magic byte")
        }
    }
}
impl Into<u8> for Magic {
    fn into(self) -> u8 {
        MAGIC_BYTE
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Version;
impl TryFrom<u8> for Version {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == SER_VERSION {
            Ok(Version)
        } else {
            Err("Invalid serialization version")
        }
    }
}
impl Into<u8> for Version {
    fn into(self) -> u8 {
        SER_VERSION
    }
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseFile {
    pub magic: Magic,
    pub version: Version,
    #[serde(flatten)]
    pub database: Database,
}

impl From<Database> for DatabaseFile {
    fn from(mut database: Database) -> Self {
        Self {
            magic: Magic::try_from(MAGIC_BYTE).unwrap(),
            version: Version::try_from(SER_VERSION).unwrap(),
            database,
        }
    }
}

impl DatabaseFile {
    pub fn save(&self, db_path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(&db_path)?;
        let bin = rmp_serde::to_vec(self)?;
        file.write_all(&bin)?;
        Ok(())
    }

    pub fn load(db_path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let file = std::fs::read(&db_path)?;
        let mut db_file: DatabaseFile = rmp_serde::from_slice(&file)?;
        db_file.database.populate_reverse_dependencies();
        Ok(db_file)
    }
}
