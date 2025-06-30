use std::{
    path::PathBuf,
    collections::HashSet,
};
use crate::id::Id;

pub struct Asset {
    pub id: Id,
    pub path: PathBuf,
    pub dependencies: HashSet<Id>,
}