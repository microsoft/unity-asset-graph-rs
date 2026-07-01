mod find_types;
pub mod qualified_name;
mod queries;
mod structure;
pub mod type_broker;

#[cfg(feature = "locstring")]
mod find_locstrings;

use crate::{Asset, parser::ParseError};
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
};
use tree_sitter::{Language, Parser};
use tree_sitter_c_sharp as cs;
use type_broker::TypeBroker;

pub static CS_LANG: LazyLock<Language> = LazyLock::new(|| cs::LANGUAGE.into());

pub fn parse(
    asset: &mut Asset,
    relative_to: Option<&PathBuf>,
    broker: &Arc<Mutex<TypeBroker>>,
) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(asset.path.as_ref().unwrap()),
        None => asset.path.as_ref().unwrap(),
    };

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return Err(ParseError {
                path: path.clone(),
                message: format!("Failed to read C# file: {}", e),
                inner: Some(Box::new(e)),
            });
        }
    };

    let len = match file.metadata() {
        Ok(meta) => meta.len() as usize,
        Err(e) => {
            return Err(ParseError {
                path: path.clone(),
                message: format!("Failed to read C# file metadata: {}", e),
                inner: Some(Box::new(e)),
            });
        }
    };

    let mut buf = Vec::with_capacity(len);
    if let Err(e) = file.read_to_end(&mut buf) {
        return Err(ParseError {
            path: path.clone(),
            message: "Failed to read C# file".into(),
            inner: Some(Box::new(e)),
        });
    }

    parse_buffer(&buf, asset, &path.clone(), broker)
}

fn parse_buffer(
    buffer: &[u8],
    asset: &mut Asset,
    path: &PathBuf,
    broker: &Arc<Mutex<TypeBroker>>,
) -> Result<Vec<Asset>, ParseError> {
    let mut def_assets = vec![];

    // load syntax tree
    let mut parser = Parser::new();
    parser.set_language(&CS_LANG).expect("Error loading C# grammar");
    let tree = parser.parse(buffer, None);
    let tree = match tree {
        Some(t) => t,
        None => {
            return Err(ParseError {
                path: path.clone(),
                message: "Failed to parse C# file".into(),
                ..Default::default()
            });
        }
    };

    find_types::find_types(&tree, buffer, asset, &mut def_assets, broker)?;

    #[cfg(feature = "locstring")]
    find_locstrings::find_locstrings(&tree, buffer, path, asset)?;

    Ok(def_assets)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::{AssetType, Id, QualifiedNameOwned, Relation};
    use pretty_assertions::assert_eq;
    use std::{
        collections::HashSet,
        fmt::{Display, Formatter, Result as FResult},
    };
    use tree_sitter::{Node, Point, Tree};

    pub fn _debug_up(node: Node, buffer: &[u8]) {
        let mut n = Some(node);
        while let Some(node) = n {
            let text = node.utf8_text(buffer).unwrap().split('\n').next().unwrap();
            if text.len() < 100 {
                println!("{}: {}", node.kind(), text);
            } else {
                println!(
                    "{}: {}...<{} bytes>",
                    node.kind(),
                    &text[..100],
                    node.end_byte() - node.start_byte() - 100
                );
            }
            n = node.parent();
        }
        println!();
    }

    pub fn _debug_down(node: Node, buffer: &[u8], max_depth: usize) {
        fn helper(node: Node, buffer: &[u8], depth: usize, max_depth: usize) {
            let indent = " ".repeat(depth);
            let kind = node.kind();
            let text = node.utf8_text(buffer).unwrap().split('\n').next().unwrap();
            if text.len() < 100 {
                println!("{indent}{kind}: {text}");
            } else {
                println!(
                    "{indent}{kind}: {}...<{} bytes>",
                    &text[..100],
                    node.end_byte() - node.start_byte() - 100
                );
            }

            if depth >= max_depth {
                return;
            }

            let mut cursor = node.walk();
            for c in node.children(&mut cursor) {
                helper(c, buffer, depth + 1, max_depth);
            }
        }
        helper(node, buffer, 0, max_depth);
    }

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    pub struct NodeLike {
        pub kind: &'static str,
        pub start_position: Point,
    }

    impl NodeLike {
        pub const fn new(kind: &'static str, row: usize, column: usize) -> Self {
            Self {
                kind,
                start_position: Point { row, column },
            }
        }
    }

    impl Display for NodeLike {
        fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
            write!(
                f,
                "{{ {kind} @ ({row},{column})",
                kind = self.kind,
                row = self.start_position.row,
                column = self.start_position.column,
            )
        }
    }

    impl PartialEq<Node<'_>> for NodeLike {
        fn eq(&self, other: &Node<'_>) -> bool {
            self.kind == other.kind() && self.start_position == other.start_position()
        }
    }

    impl From<Node<'_>> for NodeLike {
        fn from(value: Node<'_>) -> Self {
            Self {
                kind: value.kind(),
                start_position: value.start_position(),
            }
        }
    }

    pub const NS_TEST_CODE: &[u8] = include_bytes!("./csharp/test/ns_test.cs");
    pub static NS_TEST_TREE: LazyLock<Tree> = LazyLock::new(|| {
        let mut parser = Parser::new();
        parser
            .set_language(&CS_LANG)
            .expect("Failed to set language, bad lang version");
        parser.parse(NS_TEST_CODE, None).expect("Failed to read code")
    });

    pub const TYPE_TEST_CODE: &[u8] = include_bytes!("./csharp/test/type_test.cs");
    pub static TYPE_TEST_TREE: LazyLock<Tree> = LazyLock::new(|| {
        let mut parser = Parser::new();
        parser
            .set_language(&CS_LANG)
            .expect("Failed to set language, bad lang version");
        parser.parse(TYPE_TEST_CODE, None).expect("Failed to read code")
    });

    pub const VAR_TEST_CODE: &[u8] = include_bytes!("./csharp/test/var_test.cs");
    pub static VAR_TEST_TREE: LazyLock<Tree> = LazyLock::new(|| {
        let mut parser = Parser::new();
        parser
            .set_language(&CS_LANG)
            .expect("Failed to set language, bad lang version");
        parser.parse(VAR_TEST_CODE, None).expect("Failed to read code")
    });
}
