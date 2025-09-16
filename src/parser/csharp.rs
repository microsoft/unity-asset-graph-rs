use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::LazyLock,
};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_c_sharp as cs;
use crate::{
    Asset,
    AssetType,
    Id,
    parser::ParseError,
    Relation,
};

static CS_LANG: LazyLock<Language> = LazyLock::new(|| {
    cs::LANGUAGE.into()
});

/// Query to find class, struct, enum, and interface declarations.
/// Syntax tree identifiers come from https://github.com/tree-sitter/tree-sitter-c-sharp/blob/master/src/node-types.json
static CSOBJ_QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&CS_LANG, r#"
[(class_declaration
    name: (identifier) @name
)
(struct_declaration
    name: (identifier) @name
)
(enum_declaration
    name: (identifier) @name
)
(interface_declaration
    name: (identifier) @name
)]"#)
        .expect("Failed to compile class query")
});

pub fn parse(asset: &mut Asset, relative_to: Option<&PathBuf>) -> Result<Vec<Asset>, ParseError> {
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

    parse_buffer(&buf, asset, relative_to)
}

fn parse_buffer(
    buffer: &[u8], 
    asset: &mut Asset, 
    relative_to: Option<&PathBuf>,
) -> Result<Vec<Asset>, ParseError> {
    let path = match relative_to {
        Some(rel) => &rel.join(asset.path.as_ref().unwrap()),
        None => asset.path.as_ref().unwrap(),
    };

    let mut def_assets = vec![];
    
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

    let mut q = QueryCursor::new();
    let mut iter = q.matches(&CSOBJ_QUERY, tree.root_node(), buffer);
    while let Some(m) = iter.next() {
        let node = m.captures[0].node;
        let text = &buffer[node.start_byte()..node.end_byte()];
        let obj_name = match std::str::from_utf8(text) {
            Ok(name) => name,
            Err(_) => continue,
        };

        let mut parent = node.parent();
        let mut namespace = None;
        while let Some(p) = parent {
            if p.kind() == "namespace_declaration" {
                if let Some(name_node) = p.child_by_field_name("name") {
                    let name_text = &buffer[name_node.start_byte()..name_node.end_byte()];
                    match std::str::from_utf8(name_text) {
                        Ok(ns) => {
                            namespace = Some(ns);
                            break;
                        },
                        Err(_) => break,
                    }
                } else {
                    break;
                }
            }
            parent = p.parent();
        }

        let fqn = if let Some(ns) = namespace {
            format!("{ns}.{obj_name}")
        } else {
            obj_name.to_string()
        };

        let mut def = Asset {
            id: Id::CsType(fqn),
            path: None,
            asset_type: AssetType::CsDeclaration,
            ..Default::default()
        };
        def.relations.insert(Relation::ContainedBy(asset.id.clone()));

        def_assets.push(def);
    }

    Ok(def_assets)
}