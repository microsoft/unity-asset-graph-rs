use crate::{
    asset_type::AssetType,
    database::{AssetFilter, Database},
    id::Id,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::HashSet,
    fmt::{Display, Formatter, Result},
    path::PathBuf,
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

    pub fn bind<'a>(&'a self, db: &'a Database) -> Option<BoundRelation<'a>> {
        db.asset(self.id()).map(|a| match self {
            Self::Uses(id) => BoundRelation::Uses(a),
            Self::UsedBy(id) => BoundRelation::UsedBy(a),
            Self::Contains(id) => BoundRelation::Contains(a),
            Self::ContainedBy(id) => BoundRelation::ContainedBy(a),
        })
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

pub enum BoundRelation<'a> {
    Uses(BoundAsset<'a>),
    UsedBy(BoundAsset<'a>),
    Contains(BoundAsset<'a>),
    ContainedBy(BoundAsset<'a>),
}

impl<'a> BoundRelation<'a> {
    pub fn matches_type(&self, typ: &Relation) -> Option<&BoundAsset<'a>> {
        match (self, typ) {
            (Self::Uses(a), Relation::Uses(_)) => Some(a),
            (Self::UsedBy(a), Relation::UsedBy(_)) => Some(a),
            (Self::Contains(a), Relation::Contains(_)) => Some(a),
            (Self::ContainedBy(a), Relation::ContainedBy(_)) => Some(a),
            _ => None,
        }
    }

    pub fn asset(&self) -> &BoundAsset<'a> {
        match self {
            Self::Uses(a) => a,
            Self::UsedBy(a) => a,
            Self::Contains(a) => a,
            Self::ContainedBy(a) => a,
        }
    }
}

impl Display for BoundRelation<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Uses(a) => write!(f, "Uses {}", a.id()),
            Self::UsedBy(a) => write!(f, "UsedBy {}", a.id()),
            Self::Contains(a) => write!(f, "Contains {}", a.id()),
            Self::ContainedBy(a) => write!(f, "ContainedBy {}", a.id()),
        }
    }
}

#[derive(Deserialize, Serialize, Default, PartialEq, Eq, Debug, Clone)]
pub struct Asset {
    pub id: Id,
    pub asset_type: AssetType,
    pub path: Option<PathBuf>,
    pub relations: HashSet<Relation>,

    #[serde(skip)]
    pub back_relations: HashSet<Relation>,
    #[serde(skip)]
    id_cache: RefCell<Option<String>>,
}

impl Asset {
    pub fn new(id: Id, typ: AssetType, path: Option<PathBuf>, relations: impl IntoIterator<Item = Relation>) -> Self {
        Self {
            id,
            asset_type: typ,
            path,
            relations: relations.into_iter().collect(),
            back_relations: HashSet::new(),
            id_cache: RefCell::new(None),
        }
    }

    pub fn bind<'a>(&'a self, db: &'a Database) -> BoundAsset<'a> {
        BoundAsset {
            asset: self,
            db,
            indent: 0,
            path_cache: RefCell::new(None),
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

    pub fn id_matches(&self, regex: &AssetFilter) -> bool {
        let cache = self.id_cache.take().unwrap_or_else(|| self.id.to_string());
        let matches = regex.matches(&cache);
        self.id_cache.replace(Some(cache));
        matches
    }
}

pub struct BoundAsset<'a> {
    pub asset: &'a Asset,
    pub db: &'a Database,
    indent: usize,
    path_cache: RefCell<Option<&'a PathBuf>>,
}

impl<'a> BoundAsset<'a> {
    pub fn id(&self) -> &'a Id {
        &self.asset.id
    }

    pub fn asset_type(&self) -> &'a AssetType {
        &self.asset.asset_type
    }

    pub fn asset(&self) -> &'a Asset {
        &self.asset
    }

    pub fn indent(self) -> Self {
        Self {
            asset: self.asset,
            db: self.db,
            indent: self.indent + 1,
            path_cache: self.path_cache,
        }
    }

    pub fn unindent(self) -> Self {
        Self {
            asset: self.asset,
            db: self.db,
            indent: self.indent.saturating_sub(1),
            path_cache: self.path_cache,
        }
    }

    pub fn path(&self) -> Option<&'a PathBuf> {
        let mut cache = self.path_cache.take();

        if cache.is_none() {
            let mut queue = std::collections::VecDeque::from([self.asset]);
            while let Some(asset) = queue.pop_front() {
                if let Some(p) = &asset.path {
                    cache = Some(p);
                    break;
                } else {
                    let containers = asset.relations_iter().filter_map(|r| {
                        if let Relation::ContainedBy(id) = &r {
                            Some(self.db.asset(id).expect("Dangling used-by relation"))
                        } else {
                            None
                        }
                    });
                    for c in containers {
                        queue.push_back(c.asset);
                    }
                }
            }
            self.path_cache.replace(cache.clone());
        }
        cache
    }

    pub fn path_matches(&self, regex: &AssetFilter) -> bool {
        self.path()
            .map(|p| regex.matches(&p.to_string_lossy()))
            .unwrap_or(false)
    }

    pub fn relations_iter(&self) -> impl Iterator<Item = BoundRelation<'a>> {
        self.asset.relations_iter().filter_map(|r| r.bind(self.db))
    }

    pub fn display_full<'b>(&'b self) -> BoundAssetFullDisplay<'a, 'b> {
        BoundAssetFullDisplay {
            asset: self,
            ref_filter: Box::new(|_| true),
        }
    }

    pub fn display_full_filtered<'b>(
        &'b self,
        filter: impl Fn(&BoundRelation) -> bool + 'b,
    ) -> BoundAssetFullDisplay<'a, 'b>
    where
        'a: 'b,
    {
        BoundAssetFullDisplay {
            asset: self,
            ref_filter: Box::new(filter),
        }
    }

    pub fn unfiltered(_: &BoundAsset) -> bool {
        true
    }

    pub fn display_short<'b>(&'b self) -> BoundAssetShortDisplay<'a, 'b>
    where
        'a: 'b,
    {
        BoundAssetShortDisplay(self)
    }
}

impl<'a> Clone for BoundAsset<'a> {
    fn clone(&self) -> Self {
        Self {
            asset: self.asset,
            db: self.db,
            indent: self.indent,
            path_cache: self.path_cache.clone(),
        }
    }
}

impl std::hash::Hash for BoundAsset<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

impl std::cmp::PartialEq for BoundAsset<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.asset() == other.asset()
    }
}
impl std::cmp::Eq for BoundAsset<'_> {}

pub struct BoundAssetFullDisplay<'a, 'b> {
    pub asset: &'b BoundAsset<'a>,
    pub ref_filter: Box<dyn Fn(&BoundRelation) -> bool + 'b>,
}

impl BoundAssetFullDisplay<'_, '_> {
    fn fmt_relation(&self, f: &mut Formatter<'_>, relation: Relation) -> Result {
        let indent_str = "  ".repeat(self.asset.indent + 1);
        let mut deps: Vec<_> = self
            .asset
            .relations_iter()
            .filter_map(|r| {
                if r.matches_type(&relation).is_some() {
                    if (self.ref_filter)(&r) {
                        Some(r.asset().display_short().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        deps.sort();

        if deps.len() > 0 {
            writeln!(f, "{indent_str}{relation} ({}):", deps.len())?;
            for dep in &deps {
                writeln!(f, "{indent_str}- {}", dep)?;
            }
        }

        Ok(())
    }
}

impl Display for BoundAssetFullDisplay<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let first_indent = format!("{}- ", "  ".repeat(self.asset.indent));
        let indent_str = "  ".repeat(self.asset.indent + 1);
        writeln!(f, "{first_indent}Asset ID: {}", self.asset.id())?;
        writeln!(f, "{indent_str}Type: {}", self.asset.asset_type())?;
        if let Some(path) = self.asset.path() {
            writeln!(f, "{indent_str}Path: {}", path.display())?;
        }

        self.fmt_relation(f, Relation::ContainedBy(Id::None))?;
        self.fmt_relation(f, Relation::Contains(Id::None))?;
        self.fmt_relation(f, Relation::UsedBy(Id::None))?;
        self.fmt_relation(f, Relation::Uses(Id::None))?;

        Ok(())
    }
}

pub struct BoundAssetShortDisplay<'a, 'b>(pub &'b BoundAsset<'a>);
impl Display for BoundAssetShortDisplay<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match (self.0.id(), self.0.path()) {
            (Id::CsType(name), Some(p)) => {
                write!(
                    f,
                    "{name} in {p}",
                    p = p.file_name().expect("Bad path").to_str().expect("Bad path")
                )
            }
            (_, Some(p)) => {
                write!(f, "{}", p.display())
            }
            _ => {
                write!(f, "{}", self.0.id())
            }
        }
    }
}
