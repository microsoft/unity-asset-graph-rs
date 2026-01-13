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
    Asset, AssetType, Id, Relation, parser::{ParseError, QualifiedName, TypeBroker}
};
use super::queries as queries;

struct TypeInfo<'a> {
    node: Node<'a>,
    name: QualifiedName,
}

/// Find type declarations and usages in the given syntax tree, updating the provided asset and type broker accordingly.
pub fn find_types(
    tree: &Tree, 
    buffer: &[u8], 
    asset: &mut Asset, 
    def_assets: &mut Vec<Asset>, 
    broker: &Arc<Mutex<TypeBroker>>,
) -> Result<(), ParseError> {
    // println!("find_types");
    // // included namespaces from using directives
    // let mut usings = vec![];
    // // included type aliases, i.e. `using X = F.Q.N;`
    // let mut aliases = HashMap::new();

    // // first, gather using directives
    // let mut q = QueryCursor::new();
    // let mut iter = q.matches(&queries::USING_QUERY, tree.root_node(), buffer);
    // 'a: while let Some(m) = iter.next() {
    //     // this using directive defines an alias
    //     if let Some(alias) = m.nodes_for_capture_index(queries::USING_QUERY.capture_index_for_name("alias").unwrap()).next()
    //     && let Some(qn_node) = m.nodes_for_capture_index(queries::USING_QUERY.capture_index_for_name("type").unwrap()).next() {
    //         let fqn = qn_node.utf8_text(buffer).unwrap();
    //         aliases.insert(alias, QualifiedName::from(fqn));
    //     }
    //     // this using directive is a normal namespace import
    //     else {
    //         let text = m.nodes_for_capture_index(queries::USING_QUERY.capture_index_for_name("type").unwrap())
    //             .next()
    //             .unwrap()
    //             .utf8_text(buffer)
    //             .unwrap();

    //         usings.push(QualifiedName::from(text));
    //     }
    // }

    // // loop over each type declaration and create a sub-asset for it
    // let decls = find_declarations(tree, buffer);
    // for decl in &decls {
    //     let a = Asset {
    //         id: Id::CsType(decl.name.clone()),
    //         path: None,
    //         asset_type: AssetType::CsType,
    //         relations: HashSet::from([Relation::ContainedBy(asset.id.clone())]),
    //         ..Default::default()
    //     };

    //     // find all other types used by the declared type
    //     for usage in find_usages(decl.node, buffer) {
    //         // add the containing type's namespaces to the using list
    //         let mut usings = usings.clone();
    //         for i in 0 .. decl.name.len() - 1 {
    //             let ns = QualifiedName::from_iter(decl.name.iter().take(i + 1).map(|s| s.as_str()));
    //             usings.push(ns);
    //         }

    //         // the usage matches a known alias
    //         if let Some(t) = aliases.get(&usage) {
    //             broker.lock().unwrap().request(&a.id, t.clone(), &usings);
    //         } else {
    //             let name = resolve_qualified_name(usage, buffer);
    //             if name != decl.name {
    //                 broker.lock().unwrap().request(&a.id, name, &usings);
    //             }
    //         }
    //     }

    //     def_assets.push(a);
    // }

    Ok(())
}

// /// This really should work, but for some reason it doesn't
// // static DECL_SUBTYPES: LazyLock<Vec<&str>> = LazyLock::new(|| {
// //     super::CS_LANG.subtypes_for_supertype(
// //         super::CS_LANG.id_for_node_kind("type_declaration", true),
// //     ).iter().map(|k| super::CS_LANG.node_kind_for_id(*k).unwrap()).collect()
// // });
// const DECL_SUBTYPES: [&str; 6] = [
//     "class_declaration", 
//     "delegate_declaration", 
//     "enum_declaration", 
//     "interface_declaration", 
//     "record_declaration", 
//     "struct_declaration",
// ];

// fn find_declarations<'a, 'b>(
//     tree: &'a Tree,
//     buffer: &'b [u8],
// ) -> Vec<TypeInfo<'a>> {
//     println!("find_declarations");
//     let mut decls = vec![];

//     // loop over all type declarations
//     let mut q = QueryCursor::new();
//     let mut iter = q.matches(&queries::TYPE_DECL, tree.root_node(), buffer);
//     while let Some(m) = iter.next() {
//         decls.push(TypeInfo {
//             node: m.captures[0].node,
//             name: resolve_declaration(m.captures[0].node, buffer),
//         });
//     }
//     decls
// }

// /// Walk up from the decl identifier node to find the full name and namespace.
// fn resolve_declaration(decl_node: Node, buffer: &[u8]) -> QualifiedName {
//     let mut name_parts = vec![];
//     let mut node = Some(decl_node);
//     while let Some(n) = node && n.kind() != "compilation_unit" {
//         if DECL_SUBTYPES.contains(&n.kind()) {
//             name_parts.insert(0, n.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap());
//         } else if n.kind() == "namespace_declaration" {
//             let ns = n.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap();
//             for (i, part) in ns.split('.').enumerate() {
//                 name_parts.insert(i, part);
//             }
//         }
//         node = n.parent();
//     }

//     if let Some(root) = node {
//         for child in root.children(&mut root.walk()) {
//             if child.kind() == "file_scoped_namespace_declaration" {
//                 let ns = child.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap();
//                 for (i, part) in ns.split('.').enumerate() {
//                     name_parts.insert(i, part);
//                 }
//             }
//         }
//     }

//     if name_parts.is_empty() {
//         _debug(decl_node, buffer);
//         panic!("Failed to resolve declaration name");
//     }

//     QualifiedName::from_iter(name_parts)
// }

// /// Find all type usages within the given type definition node.
// fn find_usages<'a>(
//     node: Node<'a>, 
//     buffer: &'a [u8], 
// ) -> Vec<Node<'a>> {
//     let mut usages = vec![];

//     // find hard type usages
//     let mut qcursor = QueryCursor::new();
//     let mut iter = qcursor.matches(&queries::TYPE_USAGE, node, buffer);
//     while let Some(m) = iter.next() {
//         let n = m.captures[0].node;
//         if n == node {
//             continue;
//         }
//         usages.push(n);
//     }

//     // find static member usages
//     let mut qcursor = QueryCursor::new();
//     let mut iter = qcursor.matches(&queries::VAR_USAGE, node, buffer);
//     'u: while let Some(m) = iter.next() {
//         let usage = m.captures[0].node;
//         let name = usage.utf8_text(buffer).unwrap();
//         println!("Found member access usage: {name}");
        
//         // find containing scope
//         let mut cache = HashMap::new();
//         let mut parent_scope = resolve_parent_scope(usage);
//         while let Some(parent) = parent_scope && parent != node {
//             // find declared variables in this scope
//             let vars = find_vars_in_scope(parent, buffer, &mut cache);
//             println!("Vars in scope {}: {vars:?}", parent.kind());

//             // if the name matches a declared variable, it's not a type usage
//             if vars.contains(&name) {
//                 continue 'u;
//             }

//             parent_scope = resolve_parent_scope(parent);
//         }

//         usages.push(usage);
//     }
//     usages
// }

// fn resolve_parent_scope(node: Node) -> Option<Node> {
//     let mut node = node.parent();
//     while let Some(n) = node {
//         if matches!(n.kind(), "block" | "method_declaration" | "declaration_list") {
//             return Some(n);
//         }
//         node = n.parent();
//     }
//     None
// }

// fn find_vars_in_scope<'map, 'buf>(node: Node<'buf>, buffer: &'buf [u8], cache: &'map mut HashMap<Node<'buf>, Vec<&'buf str>>) -> &'map Vec<&'buf str> {
//     if !cache.contains_key(&node) {
//         let mut vars = vec![];
//         let mut q = QueryCursor::new();
//         let mut iter = q.matches(&queries::VAR_DECL, node, buffer);
//         while let Some(m) = iter.next() {
//             let n = m.captures[0].node;
//             let name = n.utf8_text(buffer).unwrap();
//             vars.push(name);
//         }
//         cache.insert(node, vars);
//     }

//     cache.get(&node).unwrap()
// }

// fn resolve_qualified_name(node: Node, buffer: &[u8]) -> QualifiedName {
//     match node.kind() {
//         "identifier" => {
//             QualifiedName::from(node.utf8_text(buffer).unwrap())
//         },
//         "qualified_name" => {
//             let mut qn = QualifiedName::from(
//                 resolve_qualified_name(
//                     node.child_by_field_name("qualifier").unwrap(),
//                     buffer,
//                 ),
//             );
//             qn.push(node.child_by_field_name("name").unwrap().utf8_text(buffer).unwrap().to_string());
//             qn
//         },
//         "generic_name" => {
//             let id = node.named_child(0).unwrap().utf8_text(buffer).unwrap();
//             let args = node.named_child(1).unwrap().named_child_count();
//             QualifiedName::from(format!("{id}<{args}>"))
//         },
//         _ => {
//             _debug(node, buffer);
//             panic!("Unexpected node kind in type usage: {}", node.kind());
//         }
//     }
// }

// fn _debug(node: Node, buffer: &[u8]) {
//     let mut n = Some(node);
//     while let Some(node) = n {
//         let text = node.utf8_text(buffer).unwrap().split('\n').next().unwrap();
//         if text.len() < 100 {
//             println!("{}: {}", node.kind(), text);
//         }
//         else {
//             println!("{}: {}...<{} bytes>", node.kind(), &text[..100], node.end_byte() - node.start_byte() - 100);
//         }
//         n = node.parent();
//     }
//     println!();
// }