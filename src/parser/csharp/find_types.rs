use std::{
    collections::HashSet,
    sync::{Arc, Mutex, LazyLock},
};
use tree_sitter::{
    Node, 
    Query, 
    QueryCapture, 
    QueryCursor, 
    StreamingIterator, 
    Tree,
};
use crate::{
    Asset, 
    AssetType, 
    Id, 
    parser::{ParseError, TypeBroker}, 
    Relation,
};

pub fn find_types(
    tree: &Tree, 
    buffer: &[u8], 
    asset: &mut Asset, 
    def_assets: &mut Vec<Asset>, 
    broker: &Arc<Mutex<TypeBroker>>,
) -> Result<(), ParseError> {
    let decls = find_declarations(tree, buffer);

    for decl in &decls {
        let (name, namespace) = resolve_declaration(*decl, buffer);
        let a = Asset {
            id: Id::CsType { name, namespace },
            path: None,
            asset_type: AssetType::CsType,
            relations: HashSet::from([Relation::ContainedBy(asset.id.clone())]),
            ..Default::default()
        };
        def_assets.push(a);

        let usages = find_usages(decl.parent().unwrap(), buffer);
        for u in usages.iter().filter(|n| !decls.contains(n)) {
            println!("Possible external type: {}", u.utf8_text(buffer).unwrap());
        }
    }

    Ok(())
}


/// Query to find class, struct, enum, and interface declarations.
/// Syntax tree identifiers come from https://github.com/tree-sitter/tree-sitter-c-sharp/blob/master/src/node-types.json
static CSOBJ_QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
[
    (class_declaration
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
    )
]"#
    ).expect("Failed to compile class query")
});

fn find_declarations<'a>(
    tree: &'a Tree,
    buffer: &'a [u8],
) -> Vec<Node<'a>> {
    let mut decls = vec![];

    // loop over all type declarations
    let mut q = QueryCursor::new();
    let mut iter = q.matches(&CSOBJ_QUERY, tree.root_node(), buffer);
    while let Some(m) = iter.next() {
        decls.push(m.captures[0].node);
    }
    decls
}

fn resolve_declaration(id_node: Node, buffer: &[u8]) -> (String, Option<String>) {
    let mut name_parts = vec![];
    let mut node = id_node.parent();
    while let Some(n) = node && n.kind() != "namespace_declaration" {
        if let "class_declaration" | "struct_declaration" | "enum_declaration" | "interface_declaration" = n.kind() {
            name_parts.push(n.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap())
        }
        node = n.parent();
    }

    let mut ns_parts: Vec<&str> = vec![];
    while let Some(n) = node {
        if n.kind() == "namespace_declaration" {
            ns_parts.push(
                n.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap(),
            );
        }
        node = n.parent();
    }

    let name = name_parts.iter().rev().cloned().collect::<Vec<&str>>().join(".");
    let ns = ns_parts.iter().rev().cloned().collect::<Vec<&str>>().join(".");
    (name, if ns_parts.is_empty() { None } else { Some(ns) })
}

static USAGE_QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
(type) @type
"#
    ).expect("Failed to compile usage query")
});

fn find_usages<'a>(
    node: Node<'a>, 
    buffer: &[u8], 
) -> Vec<Node<'a>> {
    let mut usages = vec![];

    let mut qcursor = QueryCursor::new();
    let mut iter = qcursor.matches(&USAGE_QUERY, node, buffer);
    while let Some(m) = iter.next() {
        let node = m.captures[0].node;
        if node.kind() != "predefined_type" {
            usages.push(node);
        }
    }
    usages
}

fn debug(node: Node, buffer: &[u8]) {
    let mut n = Some(node);
    while let Some(node) = n {
        if node.end_byte() - node.start_byte() < 100 {
            println!("{}: {}", node.kind(), node.utf8_text(&buffer).unwrap());
        }
        else {
            println!("{}: <{} bytes>", node.kind(), node.end_byte() - node.start_byte());
        }
        n = node.parent();
    }
}