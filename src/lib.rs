pub mod database;
mod parser;
pub mod asset;
pub mod asset_type;
pub mod id;
mod util;
pub mod version;

pub use database::{Database, DatabaseError};
pub use asset::{Asset, BoundAsset, Relation};
pub use asset_type::AssetType;
pub use id::Id;
pub use version::{DatabaseFile, Magic, Version};