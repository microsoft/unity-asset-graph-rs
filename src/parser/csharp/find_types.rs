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
    Asset, AssetType, Id, QualifiedName, Relation, parser::{ParseError, TypeBroker}
};

use super::{
    qualified_name::{QualifiedNameOwned, QualifiedNameRef, Error as NameError},
    structure::*,
    queries::kinds as k,
    type_broker::TypeRequest,
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

    let asset_map = process_declarations(&info)
        .map_err(|e| ParseError {
            path: asset.path.as_ref().unwrap().clone(),
            message: "Failed to qualify type declaration names".to_string(),
            inner: Some(Box::new(e)),
        })?;

    for name in asset_map.values() {
        def_assets.push(Asset {
            id: Id::CsType(name.to_owned()),
            asset_type: AssetType::CsType,
            relations: HashSet::from([Relation::ContainedBy(asset.id.clone())]),
            ..Default::default()
        });
    }

    let mut b = broker.lock().unwrap();
    for r in process_type_usages(&info, &asset_map) {
        b.push(r);
    }

    Ok(())
}

fn process_declarations<'t, 'b>(info: &StructureInfo<'b, 't>) -> Result<HashMap<Node<'t>, QualifiedNameRef<'b>>, NameError> {
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
                full_name = QualifiedNameRef::try_concat(ns.clone(), full_name)?;
            }
            parent = p;
        }
        
        if let Some(ref fsns) = info.fsns_decl {
            full_name = QualifiedNameRef::try_concat(fsns.clone(), full_name)?;
        }

        asset_map.insert(node.clone(), full_name);
    }

    Ok(asset_map)
}

fn process_type_usages<'b, 't>(
    info: &StructureInfo<'b, 't>, decls: &HashMap<Node<'t>, QualifiedNameRef<'b>>,
) -> HashSet<TypeRequest> {
    let mut requests = HashSet::new();

    // check all the used types against the declared types, file requests for the mismatches
    'usages: for (node, name) in info.type_usages.iter() {
        let mut container = None;
        let mut use_name = name.clone();
        let mut ns = vec![];
        let mut local_ns = vec![];

        // walk up the hierarchy looking for all the stuff
        let mut i = *node;
        while let Some(ancestor) = i.parent() {

            // skip if type is locally declared
            if let Some(scope_decls) = info.type_decl_names.get(&ancestor)
            && scope_decls.contains(&name) {
                continue 'usages;
            }

            // resolve namespace alias if found
            if let Some(ns_alias) = use_name.alias {
                if ns_alias == "global" {
                    use_name.alias = None;
                    requests.insert(TypeRequest {
                        requester: Id::CsType(name.to_owned()),
                        partial_name: use_name.to_owned(),
                        scoped_namespaces: vec![],
                    });
                }
                else if let Some(scope_aliases) = info.aliases.get(&ancestor)
                && let Some(sub) = scope_aliases.get(&QualifiedNameRef::from(ns_alias)) {
                    use_name.resolve_alias(sub.clone());
                }
            }

            // save containing class
            if let Some(decl) = decls.get(&ancestor) {
                container = container.or(Some(decl.clone()));
            }

            // save imported namespaces
            if let Some(scoped_ns) = info.ns_usages.get(&ancestor) {
                for import in scoped_ns {
                    ns.push(import.clone());
                }
            }

            // namespace declarations
            if let Some(ns_decl) = info.ns_decl_nodes.get(&ancestor) {
                // prepend newly found namespace to all the other namespaces we found in our walk
                // upward, i.e. ["Ns1"] => ["Ns0.Ns1", "Ns0"]
                local_ns = local_ns.into_iter()
                    .map(|ns| QualifiedNameRef::concat(ns_decl, ns))
                    .chain([ns_decl.clone()].into_iter())
                    .collect();
            }

            i = ancestor;
        }

        if let Some(fsns) = &info.fsns_decl {
            local_ns = local_ns.into_iter()
                .map(|ns| QualifiedNameRef::concat(fsns, ns))
                .chain([fsns.clone()].into_iter())
                .collect();
        }

        if let Some(c) = container {
            requests.insert(TypeRequest {
                requester: Id::CsType(c.to_owned()),
                partial_name: name.to_owned(),
                scoped_namespaces: ns.iter().chain(local_ns.iter()).map(|n| n.to_owned()).collect(),
            });
        }
    }

    requests
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::test::*;

    #[test]
    fn type_usages_ns() {
        let info = super::super::structure::evaluate_structure(&NS_TEST_TREE, NS_TEST_CODE).unwrap();
        let decls = process_declarations(&info).unwrap();
        let ref_types = process_type_usages(&info, &decls);

        for r in &ref_types {
            for ns in &r.scoped_namespaces {
                println!("Scoped ns: {ns}");
            }
        }

        assert_eq!(ref_types, HashSet::from([
            TypeRequest {
                requester: Id::CsType(QualifiedNameOwned::from("L0.L1.L2.Class2")),
                partial_name: QualifiedNameOwned::from("L3.Class3"),
                scoped_namespaces: [
                    "Ns2", "Ns1", "Ns0",
                    "L0.L1.L2", "L0.L1", "L0",
                ].into_iter().map(QualifiedNameOwned::from).collect(),
            }
        ]));
    }
}
