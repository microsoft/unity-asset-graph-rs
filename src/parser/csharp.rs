//mod queries;
mod find_types;
pub mod type_broker;

#[cfg(feature = "locstring")]
mod find_locstrings;

use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex, LazyLock},
};
use tree_sitter::{Language, Parser};
use tree_sitter_c_sharp as cs;
use crate::{Asset, parser::ParseError};
use type_broker::TypeBroker;

pub static CS_LANG: LazyLock<Language> = LazyLock::new(|| {
    cs::LANGUAGE.into()
});

pub fn parse(asset: &mut Asset, relative_to: Option<&PathBuf>, broker: &Arc<Mutex<TypeBroker>>) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(asset.path.as_ref().unwrap()),
        None => asset.path.as_ref().unwrap(),
    };

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(ParseError {
            path: path.clone(),
            message: format!("Failed to read C# file: {}", e),
        }),
    };

    let len = match file.metadata() {
        Ok(meta) => meta.len() as usize,
        Err(e) => return Err(ParseError {
            path: path.clone(),
            message: format!("Failed to read C# file metadata: {}", e),
        }),
    };

    let mut buf = Vec::with_capacity(len);
    if file.read_to_end(&mut buf).is_err() {
        return Err(ParseError {
            path: path.clone(),
            message: "Failed to read C# file".into(),
        });
    }

    parse_buffer(&buf, asset, &path.clone(), broker)
}

fn parse_buffer(
    buffer: &[u8], 
    asset: &mut Asset, 
    path: &PathBuf,
    broker: &Arc<Mutex<TypeBroker>>
) -> Result<Vec<Asset>, ParseError> {
    let mut def_assets = vec![];
    
    // load syntax tree
    let mut parser = Parser::new();
    parser.set_language(&CS_LANG).expect("Error loading C# grammar");
    let tree = parser.parse(buffer, None);
    let tree = match tree {
        Some(t) => t,
        None => return Err(ParseError {
            path: path.clone(),
            message: "Failed to parse C# file".into(),
        }),
    };

    find_types::find_types(&tree, buffer, asset, &mut def_assets, broker)?;

    #[cfg(feature = "locstring")]
    find_locstrings::find_locstrings(&tree, buffer, path, asset)?;

    Ok(def_assets)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;
    use crate::{AssetType, Id, Relation};

    #[test]
    fn test_parse_csharp() -> Result<(), ParseError> {
        let code = r#"
using System;
using My.DifferentNamespace;

namespace My.Namespace {
    public class MyClass {
        internal class UnderClass { }

        private static My.OtherNamespace.LocalizedString locstringNormal = LocStringCache.Get("NormalKey");

        private static LocalizedString locstringPrefixed = LocStringCache.Get(
            key: "PrefixedKey",
            formatArgs: "Some other text");

        private static LocalizedString locstringBad = LocStringCache.Get(someKey);

        private static LocalizedString locstringBadPrefix = LocStringCache.Get(key: someKey);

        public int MyProperty { get; set; }
    }

    struct MyStruct {
        public int X;
        public int Y;
    }

    enum MyEnum {
        First,
        Second,
        Third
    }

    interface IMyInterface {
        void DoSomething();
    }

    namespace InnerNamespace {
        class InnerClass { }
    }
}
"#;
        let mut asset = Asset {
            asset_type: AssetType::CsFile,
            ..Default::default()
        };
        let broker = Arc::new(Mutex::new(TypeBroker::new()));
        let more_assets = parse_buffer(code.as_bytes(), &mut asset, &"no_path".into(), &broker)?;
        let broker = Arc::into_inner(broker).unwrap().into_inner().unwrap();

        assert_eq!(asset.relations, HashSet::from([
            Relation::Uses(Id::Loc("NormalKey".into())),
            Relation::Uses(Id::Loc("PrefixedKey".into()))
        ]));

        let more_reference = vec![
            Asset {
                id: Id::CsType { name: "MyClass".into(), namespace: Some("My.Namespace".into()) },
                asset_type: AssetType::CsType,
                relations: HashSet::from([
                    Relation::ContainedBy(Id::None),
                ]),
                ..Default::default()
            },
            Asset {
                id: Id::CsType { name: "MyClass.UnderClass".into(), namespace: Some("My.Namespace".into()) },
                asset_type: AssetType::CsType,
                relations: HashSet::from([
                    Relation::ContainedBy(Id::None),
                ]),
                ..Default::default()
            },
            Asset {
                id: Id::CsType { name: "MyStruct".into(), namespace: Some("My.Namespace".into()) },
                asset_type: AssetType::CsType,
                relations: HashSet::from([
                    Relation::ContainedBy(Id::None),
                ]),
                ..Default::default()
            },
            Asset {
                id: Id::CsType { name: "MyEnum".into(), namespace: Some("My.Namespace".into()) },
                asset_type: AssetType::CsType,
                relations: HashSet::from([
                    Relation::ContainedBy(Id::None),
                ]),
                ..Default::default()
            },
            Asset {
                id: Id::CsType { name: "IMyInterface".into(), namespace: Some("My.Namespace".into()) },
                asset_type: AssetType::CsType,
                relations: HashSet::from([
                    Relation::ContainedBy(Id::None),
                ]),
                ..Default::default()
            },
            Asset {
                id: Id::CsType { name: "InnerClass".into(), namespace: Some("My.Namespace.InnerNamespace".into()) },
                asset_type: AssetType::CsType,
                relations: HashSet::from([
                    Relation::ContainedBy(Id::None),
                ]),
                ..Default::default()
            },
        ];
        for (i, a) in more_assets.iter().enumerate() {
            assert_eq!(a, more_reference.get(i).unwrap());
        }

        let requests_ref = HashSet::from([
            type_broker::TypeRequest::new(
                &Id::CsType { name: "MyClass".into(), namespace: Some("My.Namespace".into()) },
                "LocalizedString",
                &vec!["My.OtherNamespace".into()],
                true,
            ),
            type_broker::TypeRequest::new(
                &Id::CsType { name: "MyClass".into(), namespace: Some("My.Namespace".into()) },
                "LocalizedString",
                &vec!["My.DifferentNamespace".into(), "My.Namespace".into()],
                false,
            ),
            type_broker::TypeRequest::new(
                &Id::CsType { name: "MyClass".into(), namespace: Some("My.Namespace".into()) },
                "LocStringCache",
                &vec!["My.DifferentNamespace".into(), "My.Namespace".into()],
                false,
            ),
        ]);
        assert_eq!(broker.requests().difference(&requests_ref).collect::<Vec<&type_broker::TypeRequest>>(), Vec::<&type_broker::TypeRequest>::new());

        Ok(())
    }
}