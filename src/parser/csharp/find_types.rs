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
    Asset, AssetType, Id, Relation, parser::{ParseError, TypeBroker}
};

use super::{
    qualified_name::{QualifiedNameOwned, QualifiedNameRef},
    structure::*,
    queries::kinds as k,
};

/// Find type declarations and usages in the given syntax tree, updating the provided asset and type broker accordingly.
pub fn find_types(
    tree: &Tree, 
    buffer: &[u8], 
    asset: &mut Asset, 
    def_assets: &mut Vec<Asset>, 
    broker: &Arc<Mutex<TypeBroker>>,
) -> Result<(), ParseError> {
    let info = evaluate_structure(tree, buffer)
        .map_err(|e| ParseError {
            path: asset.path.as_ref().unwrap().clone(),
            message: "Failed to analyze structure of C# file".to_string(),
            inner: Some(Box::new(e)),
        })?;

    // identify non-nested types, create assets for them
    let mut asset_map = HashMap::new();

    'decls: for (node, name) in &info.type_decl_nodes {
        // walk up the node tree from the type decl
        let mut full_name = name.clone();
        let mut parent = node.clone();
        while let Some(p) = parent.parent() {
            // do not record assets for nested types
            if let Some(_) = info.type_decl_nodes.get(&p) {
                continue 'decls;
            }
            // if we find a namespace declaration, add it to the fully-qualified type name
            else if let Some(ns) = info.ns_decl_nodes.get(&p) {
                full_name = QualifiedNameRef::try_concat(ns.clone(), full_name)
                    .map_err(|e| ParseError {
                        path: asset.path.as_ref().unwrap().clone(),
                        message: "Failed to join qualified names".to_string(),
                        inner: Some(Box::new(e)),
                    })?;
            }
            parent = p;
        }
        if let Some(ref fsns) = info.fsns_decl {
            full_name = QualifiedNameRef::try_concat(fsns.clone(), full_name).map_err(|e| ParseError {
                path: asset.path.as_ref().unwrap().clone(),
                message: "Failed to join qualified names".to_string(),
                inner: Some(Box::new(e)),
            })?;
        }

        asset_map.insert(node, full_name);
    }

    for name in asset_map.values() {
        def_assets.push(Asset {
            id: Id::CsType(name.to_owned()),
            ..Default::default()
        });
    }

    // check all the used types against the declared types, file requests for the mismatches
    'usages: for (node, name) in info.type_usages {
        let mut container_asset = None;
        // walk up the node tree to check context
        let mut parent = node;
        while let Some(p) = parent.parent() {
            // if the type name is locally declared, discard
            if let Some(scope) = info.type_decl_names.get(&p)
            && scope.contains(&name) {
                continue 'usages;
            }
            // if the usage is within a top-level type, save it for the broker request
            container_asset = container_asset.or(asset_map.get_key_value(&p));
            parent = p;
        }

        // if the usage was within a defined asset
        if let Some((decl, decl_name)) = container_asset {
            // find all the namespaces in scope
            let mut ns = HashSet::new();
            let mut parent = **decl;
            while let Some(p) = parent.parent() {
                if let Some(scope) = info.ns_usages.get(&p) {
                    ns.union(scope);
                }
                parent = p;
            }

            // file request
            let ns = ns.iter().map(|n| n.to_owned()).collect::<HashSet<QualifiedNameOwned>>();
            let b = &mut broker.lock().unwrap();
            b.request(&Id::CsType(decl_name.to_owned()), name.to_owned(), ns);
        }
    }

    Ok(())
}
