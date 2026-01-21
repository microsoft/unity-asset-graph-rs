use std::{
    collections::{HashMap, HashSet}, fmt::{Display, Formatter, Result as FResult}, str::Utf8Error, sync::LazyLock
};
use tree_sitter::{Tree, Query, QueryCursor, QueryError, QueryMatch, Node, StreamingIterator};
use crate::parser::csharp::qualified_name::{self, QualifiedNameRef};

use super::{
    CS_LANG,
    queries::QUERY_ALL,
    qualified_name::QualifiedName,
};

#[derive(Debug)]
pub enum Error<'a> {
    Query(QueryError),
    FieldName(&'a str),
    FieldId(u32),
    Utf8(Utf8Error),
    BadStaticUsing(&'a str),
    BadName(qualified_name::Error<'a>),
    Unknown(&'a str),
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        match self {
            Self::Query(q) => write!(f, "{q}"),
            Self::FieldName(e) => write!(f, "No such field '{e}'"),
            Self::FieldId(id) => write!(f, "No such field {id}"),
            Self::Utf8(e) => write!(f, "Failed to convert buffer to UTF-8: {e}"),
            Self::BadStaticUsing(s) => write!(f, "Failed to parse static using '{s}'"),
            Self::BadName(e) => write!(f, "Failed to parse name: {e}"),
            Self::Unknown(s) => write!(f, "Unknown error: {s}"),
        }
    }
}

impl<'a> std::error::Error for Error<'a> {}

static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&CS_LANG, QUERY_ALL).expect("Failed to compile query")
});

#[derive(Default)]
pub struct StructureInfo<'buffer, 'tree> {
    pub namespaces: HashSet<&'buffer str>,
    pub aliases: HashMap<&'buffer str, &'buffer str>,
    pub type_declarations: HashSet<QualifiedNameRef<'buffer>>,
    pub id_scopes: HashMap<Node<'tree>, HashSet<&'buffer str>>,
    pub id_uses: HashMap<Node<'tree>, QualifiedNameRef<'buffer>>,
}

pub fn evaluate_structure<'t, 'b>(tree: &'t Tree, buffer: &'b [u8]) -> Result<StructureInfo<'b, 't>, Error<'b>> {
    let mut results = StructureInfo { ..Default::default() };

    let f_ns_use = get_field(&QUERY, "ns_use")?;
    let f_type_decl = get_field(&QUERY, "type_decl")?;
    let f_var_decl = get_field(&QUERY, "var_decl")?;
    let f_type_use = get_field(&QUERY, "type_use")?;
    let f_var_use = get_field(&QUERY, "var_use")?;
    let f_id = get_field(&QUERY, "id")?;

    let mut cursor = QueryCursor::new();
    let mut iter = cursor.matches(&QUERY, tree.root_node(), buffer);

    while let Some(m) = iter.next() {
        for c in m.captures {
            if c.index == f_ns_use {
                evaluate_ns(c.node, m, buffer, &mut results)?;
            } else if c.index == f_type_decl {
                evaluate_type_decl(c.node, m, buffer, &mut results)?;
            } else if c.index == f_var_decl {
                evaluate_var_decl(c.node, m, buffer, &mut results)?;
            } else if c.index == f_type_use {
                evaluate_type_use(c.node, m, buffer, &mut results)?;
            } else if c.index == f_var_use {
                evaluate_var_use(c.node, m, buffer, &mut results)?;
            } else if c.index != f_id {
                return Err(Error::FieldId(c.index));
            }
        }
    }

    Ok(results)
}

fn get_field<'q, 'f>(query: &'q Query, field: &'f str) -> Result<u32, Error<'f>> {
    match query.capture_index_for_name(field) {
        Some(f) => Ok(f),
        None => Err(Error::FieldName(field)),
    }
}

fn get_root(node: Node) -> Node {
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    root
}

fn evaluate_ns<'t, 'b>(
    node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error<'b>> {
    let f_id = get_field(&QUERY, "id")?;

    let id = match qmatch.nodes_for_capture_index(f_id).next() {
        Some(id) => id.utf8_text(buffer).map_err(|e| Error::Utf8(e))?,
        None => return Err(Error::Unknown(node.utf8_text(buffer).map_err(|e| Error::Utf8(e))?)),
    };

    let mut cursor = node.walk();
    let is_static = node.children(&mut cursor).any(|n| n.kind() == "static");

    if is_static {
        // `using static N.S.Type.Field;`
        // `N.S.Type`: the type actually being used when field is used
        // `Field`: the file-scoped variable that refers to the type
        let root = get_root(node);
        let (qualtype, field) = match id.rsplit_once('.') {
            Some(p) => p,
            None => return Err(Error::BadStaticUsing(
                node.utf8_text(buffer).map_err(|e| Error::Utf8(e))?
            )),
        };
        result.aliases.insert(field, qualtype);
        result.id_scopes.entry(root).or_insert(HashSet::new()).insert(field);
    } else {
        result.namespaces.insert(id);
    }

    Ok(())
}

fn evaluate_type_decl<'t, 'b>(
    node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error<'b>> {
    let ns_kind = CS_LANG.id_for_node_kind("namespace_declaration", true);
    let fsns_kind = CS_LANG.id_for_node_kind("file_scoped_namespace_declaration", true);
    let f_id = get_field(&QUERY, "id")?;

    let mut name = match qmatch.nodes_for_capture_index(f_id).next() {
        Some(id) => {
            QualifiedNameRef::try_from(id, buffer).map_err(|e| Error::BadName(e))?
        },
        None => {
            return Err(Error::Unknown(node.utf8_text(buffer).map_err(|e| Error::Utf8(e))?))
        },
    };

    // find full namespace of the declared type

    // walk up ancestor nodes, prepending any namespace declarations we come across
    let mut i = node;
    while let Some(ancestor) = i.parent() {
        if ancestor.kind_id() == ns_kind
        && let Some(ns) = ancestor.child_by_field_name("name") {
            let ns = QualifiedNameRef::try_from(ns, buffer).map_err(|e| Error::BadName(e))?;
            name = QualifiedNameRef::concat(ns, name);
        }

        i = ancestor;
    }

    // if there is a file-scoped namespace declaration, add it as well
    let root = i;
    let mut cursor = root.walk();
    if let Some(fsns) = root.named_children(&mut cursor)
        .filter(|c| c.kind_id() == fsns_kind)
        .next()
    && let Some(ns) = fsns.child_by_field_name("name")
    && let Ok(ns) = QualifiedNameRef::try_from(ns, buffer) {
        name = QualifiedNameRef::concat(ns, name);
    }

    result.type_declarations.insert(name);

    Ok(())
}


fn evaluate_var_decl<'t, 'b>(
    node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error<'b>> {
    let f_id = get_field(&QUERY, "id")?;
    let id = match qmatch.nodes_for_capture_index(f_id).next() {
        Some(id) => id,
        None => return Err(Error::Unknown(node.utf8_text(buffer).map_err(|e| Error::Utf8(e))?)),
    };

    result.id_scopes.entry(node)
        .or_insert(HashSet::new())
        .insert(id.utf8_text(buffer).map_err(|e| Error::Utf8(e))?);
    Ok(())
}

fn evaluate_type_use<'t, 'b>(
    node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error<'b>> {
    Ok(())
}

fn evaluate_var_use<'t, 'b>(
    node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error<'b>> {
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        sync::LazyLock,
    };
    use tree_sitter::{Parser, Point};
    use crate::parser::csharp::{
        CS_LANG,
        test::{NodeLike, TYPE_TEST_CODE, TYPE_TEST_TREE},
    };
    use super::*;

    #[test]
    fn evaluate_structure() -> Result<(), Error<'static>> {
        let mut result = super::evaluate_structure(&TYPE_TEST_TREE, TYPE_TEST_CODE)?;

        assert_eq!(result.namespaces, HashSet::from([
            "X", "System.Text",
        ]));

        assert_eq!(result.aliases, HashMap::from([
            ("StaticField", "X.Y.Z.Class"),
        ]));

        assert_eq!(result.type_declarations, HashSet::from([
            QualifiedNameRef::from("A.B.ClassB"), QualifiedNameRef::from("A.B.C.ClassC"),
        ]));

        let scopes = HashMap::from([
            // file scope
            (NodeLike { kind: "compilation_unit", start_position: Point { row: 0, column: 0 }},
                HashSet::from(["XYC", "StaticField"])
            ),
            // namespace B
            (NodeLike { kind: "declaration_list", start_position: Point { row: 6, column: 12 }},
                HashSet::from(["ClassB"]),
            ),
            // ClassB
            (NodeLike { kind: "declaration_list", start_position: Point { row: 7, column: 24 }},
                HashSet::from(["Ap", "InnerClass", "B", "Method", "Delegate", "A"]),
            ),
            // ClassB[x]
            (NodeLike { kind: "indexer_declaration", start_position: Point { row: 16, column: 8 }},
                HashSet::from(["x"]),
            ),
            // ClassB.Method decl
            (NodeLike { kind: "method_declaration", start_position: Point { row: 36, column: 8 }},
                HashSet::from(["a", "b", "c"]),
            ),
            // ClassB.Method body
            (NodeLike { kind: "block", start_position: Point { row: 37, column: 8 }},
                HashSet::from(["c", "bp", "sb", "b", "poolobj"]),
            ),
            // for statement
            (NodeLike { kind: "for_statement", start_position: Point { row: 45, column: 16 }},
                HashSet::from(["i"]),
            ),
            // namespace C
            (NodeLike { kind: "declaration_list", start_position: Point { row: 54, column: 16 }},
                HashSet::from(["ClassC"]),
            ),
        ]);

        for (scope, ids) in scopes.iter() {
            let (_, node_ids) = result.id_scopes
                .extract_if(|node, _| scope == node)
                .next().expect(&format!("No matching scope for {scope:?}"));
            
            assert_eq!(ids, &node_ids);
        }
        assert_eq!(result.id_scopes, HashMap::new());

        Ok(())
    }
}