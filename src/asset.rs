use std::{
    collections::HashSet,
    fmt::{Display, Formatter, Result},
    path::PathBuf,
};
use serde::{Deserialize, Serialize};
use crate::{
    asset_type::AssetType,
    id::Id,
    database::Database,
};

#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum Relation {
    Uses(Id),
    UsedBy(Id),
    Contains(Id),
    ContainedBy(Id),
}

impl Relation {
    pub fn id(&self) -> &Id {
        match self {
            Self::Uses(id) => id,
            Self::UsedBy(id) => id,
            Self::Contains(id) => id,
            Self::ContainedBy(id) => id,
        }
    }

    pub fn matches_type(&self, other: &Relation) -> Option<&Id> {
        match (self, other) {
            (Self::Uses(id), Self::Uses(_)) => Some(id),
            (Self::UsedBy(id), Self::UsedBy(_)) => Some(id),
            (Self::Contains(id), Self::Contains(_)) => Some(id),
            (Self::ContainedBy(id), Self::ContainedBy(_)) => Some(id),
            _ => None,
        }
    }
}

impl Display for Relation {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Uses(_) => write!(f, "Uses"),
            Self::UsedBy(_) => write!(f, "Used By"),
            Self::Contains(_) => write!(f, "Contains"),
            Self::ContainedBy(_) => write!(f, "Contained By"),
        }
    }
}

#[derive(Deserialize, Serialize, Default, PartialEq, Eq, Debug)]
pub struct Asset {
    pub id: Id,
    pub asset_type: AssetType,
    pub path: Option<PathBuf>,
    pub relations: HashSet<Relation>,

    #[serde(skip)]
    pub back_relations: HashSet<Relation>,
}

impl Asset {
    pub fn bind<'a, 'b>(&'a self, db: &'b Database) -> BoundAsset<'a, 'b> {
        BoundAsset {
            asset: self,
            db,
            indent: 0,
        }
    }

    pub fn invert_relation(&self, relation: &Relation) -> Relation {
        match relation {
            Relation::Uses(_) => Relation::UsedBy(self.id.clone()),
            Relation::UsedBy(_) => Relation::Uses(self.id.clone()),
            Relation::Contains(_) => Relation::ContainedBy(self.id.clone()),
            Relation::ContainedBy(_) => Relation::Contains(self.id.clone()),
        }
    }

    pub fn relations_iter(&self) -> impl Iterator<Item = &Relation> {
        self.relations.iter().chain(self.back_relations.iter())
    }
}

pub struct BoundAsset<'a, 'b> {
    pub asset: &'a Asset,
    pub db: &'b Database,
    indent: usize,
}

impl<'a, 'b> BoundAsset<'a, 'b> {
    pub fn indent(self) -> Self {
        Self {
            asset: self.asset,
            db: self.db,
            indent: self.indent + 1,
        }
    }

    pub fn unindent(self) -> Self{
        Self {
            asset: self.asset,
            db: self.db,
            indent: self.indent.saturating_sub(1),
        }
    }

    fn fmt_relation(&self, f: &mut Formatter<'_>, relation: Relation) -> Result {
        let indent_str = "  ".repeat(self.indent + 1);
        let mut deps: Vec<String> = self.asset.relations_iter()
            .filter_map(|r| r.matches_type(&relation))
            .map(|id| {
                if let Some(dep_asset) = self.db.asset(id) {
                    if let Some(path) = &dep_asset.path {
                        path.display().to_string()
                    }
                    else {
                        dep_asset.id.to_string()
                    }
                }
                else {
                    id.to_string()
                }
            }).collect();
        deps.sort();

        writeln!(f, "{indent_str}{relation} ({}):", deps.len())?;
        for dep in &deps {
            writeln!(f, "{indent_str}- {}", dep)?;
        }

        Ok(())
    }
}

impl<'a, 'b> Display for BoundAsset<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let first_indent = format!("{}- ", "  ".repeat(self.indent));
        let indent_str = "  ".repeat(self.indent + 1);
        writeln!(f, "{first_indent}Asset ID: {}", self.asset.id)?;
        writeln!(f, "{indent_str}Type: {}", self.asset.asset_type)?;
        if let Some(path) = &self.asset.path {
            writeln!(f, "{indent_str}Path: {}", path.display())?;
        }

        self.fmt_relation(f, Relation::ContainedBy(Id::None))?;
        self.fmt_relation(f, Relation::Contains(Id::None))?;
        self.fmt_relation(f, Relation::UsedBy(Id::None))?;
        self.fmt_relation(f, Relation::Uses(Id::None))?;

        Ok(())
    }
}