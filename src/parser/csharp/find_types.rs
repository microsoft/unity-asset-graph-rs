use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, LazyLock},
};
use tree_sitter::{
    Node,
    Query,
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

const EXCLUDED_NS: [&str; 3] = [
    "System.",
    "UnityEngine.",
    "UnityEditor.",
];

struct TypeInfo<'a> {
    node: Node<'a>,
    name: String,
    namespace: Option<String>,
}

impl<'a> std::convert::Into<String> for TypeInfo<'a> {
    fn into(self) -> String {
        if let Some(ns) = self.namespace {
            format!("{ns}.{}", self.name)
        }
        else {
            self.name
        }
    }
}

static USING_QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
[
    (using_directive
        name: (identifier) @alias
        (type) @type
    )
    (using_directive
        (qualified_name) @type
        !name
    )
]"#
    ).expect("Failed to compile using query")
});

pub fn find_types(
    tree: &Tree, 
    buffer: &[u8], 
    asset: &mut Asset, 
    def_assets: &mut Vec<Asset>, 
    broker: &Arc<Mutex<TypeBroker>>,
) -> Result<(), ParseError> {
    let mut usings = vec![];
    let mut aliases = HashMap::new();

    let mut q = QueryCursor::new();
    let mut iter = q.matches(&USING_QUERY, tree.root_node(), buffer);
    'a: while let Some(m) = iter.next() {
        if let Some(alias) = m.nodes_for_capture_index(USING_QUERY.capture_index_for_name("alias").unwrap()).next()
        && let Some(fqn_node) = m.nodes_for_capture_index(USING_QUERY.capture_index_for_name("type").unwrap()).next() {
            let fqn = fqn_node.utf8_text(buffer).unwrap();
            if let Some((namespace, name)) = fqn.rsplit_once(".") {
                aliases.insert(alias, Id::CsType { name: name.into(), namespace: Some(namespace.into()) });
            }
            else {
                aliases.insert(alias, Id::CsType { name: fqn.into(), namespace: None });
            }
        }
        else {
            let text = m.nodes_for_capture_index(USING_QUERY.capture_index_for_name("type").unwrap())
                .next()
                .unwrap()
                .utf8_text(buffer)
                .unwrap();

            for exns in EXCLUDED_NS {
                if text.starts_with(exns) {
                    break 'a;
                }
            }
            usings.push(text.into());
        }
    }

    let decls = find_declarations(tree, buffer);
    for decl in &decls {
        let a = Asset {
            id: Id::CsType { name: decl.name.clone(), namespace: decl.namespace.clone() },
            path: None,
            asset_type: AssetType::CsType,
            relations: HashSet::from([Relation::ContainedBy(asset.id.clone())]),
            ..Default::default()
        };

        for usage in find_usages(decl.node, buffer) {
            if let Some(t) = aliases.get(&usage) {
                broker.lock().unwrap().request_known(&a.id, t);
            }
            else if usage.kind() == "qualified_name" {
                broker.lock().unwrap().request_known(&a.id, &resolve_qualified_name(usage, buffer))
            }
            else {
                let mut usings = usings.clone();
                if let Some(n) = &decl.namespace {
                    usings.push(n.clone())
                }
                let text = usage.utf8_text(buffer).unwrap();
                if text != decl.name {
                    broker.lock().unwrap().request(&a.id, text, &usings);
                }
            }
        }

        def_assets.push(a);
    }

    Ok(())
}

/// Query to find class, struct, enum, and interface declarations.
/// Syntax tree identifiers come from https://github.com/tree-sitter/tree-sitter-c-sharp/blob/master/src/node-types.json
static CSOBJ_QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
[
    (class_declaration)
    (struct_declaration)
    (enum_declaration)
    (interface_declaration)
] @decl"#
    ).expect("Failed to compile class query")
});

fn find_declarations<'a, 'b>(
    tree: &'a Tree,
    buffer: &'b [u8],
) -> Vec<TypeInfo<'a>> {
    let mut decls = vec![];

    // loop over all type declarations
    let mut q = QueryCursor::new();
    let mut iter = q.matches(&CSOBJ_QUERY, tree.root_node(), buffer);
    while let Some(m) = iter.next() {
        let (name, namespace) = resolve_declaration(m.captures[0].node, buffer);
        decls.push(TypeInfo {
            node: m.captures[0].node,
            name,
            namespace,
        });
    }
    decls
}

/// Walk up from the decl identifier node to find the full name and namespace.
fn resolve_declaration(decl_node: Node, buffer: &[u8]) -> (String, Option<String>) {
    let mut name_parts = vec![];
    let mut node = Some(decl_node);
    while let Some(n) = node && n.kind() != "namespace_declaration" {
        if let "class_declaration" | "struct_declaration" | "enum_declaration" | "interface_declaration" = n.kind() {
            name_parts.push(n.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap())
        }
        node = n.parent();
    }

    let mut ns_parts: Vec<&str> = vec![];
    while let Some(n) = node && n.kind() != "compilation_unit" {
        if n.kind() == "namespace_declaration" {
            ns_parts.push(
                n.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap(),
            );
        }
        node = n.parent();
    }

    if let Some(root) = node {
        for child in root.children(&mut root.walk()) {
            if child.kind() == "file_scoped_namespace_declaration" {
                ns_parts.push(
                    child.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap(),
                );
            }
        }
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
    buffer: &'a [u8], 
) -> Vec<Node<'a>> {
    let mut usages = vec![];

    let mut qcursor = QueryCursor::new();
    let mut iter = qcursor.matches(&USAGE_QUERY, node, buffer);
    while let Some(m) = iter.next() {
        let n = m.captures[0].node;
        if n != node && n.kind() != "predefined_type" {
            usages.push(n);
        }
    }
    usages
}

fn resolve_qualified_name(node: Node, buffer: &[u8]) -> Id {
    if node.kind() != "qualified_name" {
        panic!();
    }

    let name = node.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap();
    let ns = node.child_by_field_name("qualifier").unwrap().utf8_text(buffer).unwrap();
    Id::CsType { name: name.into(), namespace: Some(ns.into()) }
}

fn _debug(node: Node, buffer: &[u8]) {
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