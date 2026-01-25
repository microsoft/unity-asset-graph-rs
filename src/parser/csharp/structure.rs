use std::{
    collections::{HashMap, HashSet}, fmt::{Display, Formatter, Result as FResult}, str::Utf8Error, sync::LazyLock
};
use tree_sitter::{Tree, Query, QueryCursor, QueryError, QueryMatch, Node, StreamingIterator};
use crate::parser::csharp::qualified_name::{self, QualifiedNameRef};

use super::{
    CS_LANG,
    queries::QUERY_ALL,
};

#[derive(Debug)]
pub enum Error {
    Query(QueryError),
    FieldName(&'static str),
    FieldId(u32),
    Utf8(Utf8Error),
    BadStaticUsing(String),
    BadName(qualified_name::Error),
    Unknown(String),
}

impl Display for Error {
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

impl std::error::Error for Error {}

static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&CS_LANG, QUERY_ALL).expect("Failed to compile query")
});

static F_NS_DECL: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("ns_decl").expect("Failed to get field ns_decl"));
static F_NS_USE: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("ns_use").expect("Failed to get field ns_use"));
static F_TYPE_DECL: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("type_decl").expect("Failed to get field type_decl"));
static F_TYPE_USE: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("type_use").expect("Failed to get field type_use"));
static F_VAR_DECL: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("var_decl").expect("Failed to get field var_decl"));
static F_VAR_USE: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("var_use").expect("Failed to get field var_use"));
static F_ID: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("id").expect("Failed to get field id"));
static F_ALIAS: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("alias").expect("Failed to get field alias"));
static F_GENERICS: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("generics").expect("Failed to get field generics"));

static K_FILE_SCOPED_NS_DECL: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("file_scoped_namespace_declaration", true));
static K_USING: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("using_directive", true));
static K_STATIC: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("static", false));

#[derive(Default)]
pub struct StructureInfo<'buffer, 'tree> {
    /// A map of scope nodes to alias/original names
    pub aliases: HashMap<Node<'tree>, HashMap<QualifiedNameRef<'buffer>, QualifiedNameRef<'buffer>>>,

    /// The file-scoped namespace declaration, if any
    pub fsns_decl: Option<QualifiedNameRef<'buffer>>,

    /// A map of scope nodes to declared namespace names
    pub ns_decl: HashMap<Node<'tree>, HashSet<QualifiedNameRef<'buffer>>>,

    /// A map of scope nodes to used namespace names
    pub ns_usages: HashMap<Node<'tree>, HashSet<QualifiedNameRef<'buffer>>>,

    /// A map of scope nodes to declared type names
    pub type_decl: HashMap<Node<'tree>, HashSet<QualifiedNameRef<'buffer>>>,

    /// A map of usage nodes to the used type name
    pub type_usages: HashMap<Node<'tree>, QualifiedNameRef<'buffer>>,

    /// A map of scope nodes to declared variable names
    pub var_decl: HashMap<Node<'tree>, HashSet<QualifiedNameRef<'buffer>>>,

    /// A map of usage nodes to the used variable name
    pub var_usages: HashMap<Node<'tree>, QualifiedNameRef<'buffer>>,
}

pub fn evaluate_structure<'t, 'b>(tree: &'t Tree, buffer: &'b [u8]) -> Result<StructureInfo<'b, 't>, Error> {
    let mut results = StructureInfo { ..Default::default() };
    let mut cursor = QueryCursor::new();
    let mut iter = cursor.matches(&QUERY, tree.root_node(), buffer);

    while let Some(m) = iter.next() {
        for c in m.captures {
            if c.index == *F_NS_DECL {
                evaluate_ns_decl(c.node, m, buffer, &mut results)?;
            } else if c.index == *F_NS_USE {
                evaluate_ns_usage(c.node, m, buffer, &mut results)?;
            } else if c.index == *F_TYPE_DECL {
                evaluate_type_decl(c.node, m, buffer, &mut results)?;
            } else if c.index == *F_TYPE_USE {
                evaluate_type_usage(c.node, m, buffer, &mut results)?;
            } else if c.index == *F_VAR_DECL {
                evaluate_var_decl(c.node, m, buffer, &mut results)?;
            } else if c.index == *F_VAR_USE {
                evaluate_var_usage(c.node, m, buffer, &mut results)?;
            } else if c.index != *F_ID && c.index != *F_ALIAS && c.index != *F_GENERICS {
                return Err(Error::FieldId(c.index));
            }
        }
    }

    Ok(results)
}

fn get_root(node: Node) -> Node {
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    root
}

fn evaluate_ns_decl<'t, 'b>(
    node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error> {
    let id_node = match qmatch.nodes_for_capture_index(*F_ID).next() {
        Some(id) => id,
        None => return Err(Error::FieldName("id")),
    };
    let id = QualifiedNameRef::try_from(id_node, buffer).map_err(|e| Error::BadName(e))?;

    let decl_node = match id_node.parent() {
        Some(p) => p,
        None => {
            return Err(
                Error::Unknown(
                    id_node.utf8_text(buffer)
                        .map(|s| s.to_string())
                        .map_err(|e| Error::Utf8(e))?
                )
            );
        },
    };

    if decl_node.kind_id() == *K_FILE_SCOPED_NS_DECL {
        result.fsns_decl = Some(id);
    } else {
        result.ns_decl.entry(node).or_insert(HashSet::new()).insert(id);
    }
    Ok(())
}

fn evaluate_ns_usage<'t, 'b>(
    scope_node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error> {
    let id_node = match qmatch.nodes_for_capture_index(*F_ID).next() {
        Some(id) => id,
        None => return Err(Error::FieldName("id")),
    };
    let id = QualifiedNameRef::try_from(id_node, buffer).map_err(|e| Error::BadName(e))?;

    let alias = match qmatch.nodes_for_capture_index(*F_ALIAS).next() {
        Some(n) => Some(QualifiedNameRef::try_from(n, buffer).map_err(|e| Error::BadName(e))?),
        None => None,
    };

    let decl_node = match id_node.parent() {
        Some(p) => p,
        None => {
            return Err(
                Error::BadStaticUsing(
                    id_node.utf8_text(buffer)
                        .map(|s| s.to_string())
                        .map_err(|e| Error::Utf8(e))?
                )
            );
        },
    };
    let mut cursor = decl_node.walk();
    let is_static = decl_node.children(&mut cursor).any(|c| c.kind_id() == *K_STATIC);

    if let Some(alias) = alias {
        result.aliases.entry(scope_node).or_insert(HashMap::new())
            .insert(alias, id);
    } else if is_static {
        // `using static N.S.Type.Field;`
        // `N.S.Type`: the type actually being used when field is used
        // `Field`: the variable that refers to the type
        let mut qualtype = id;
        let field = qualtype.split_off(qualtype.len() - 1);
        result.aliases.entry(scope_node).or_insert(HashMap::new())
            .insert(field, qualtype);
    } else {
        result.ns_usages.entry(scope_node).or_insert(HashSet::new())
            .insert(id);
    }

    Ok(())
}

fn evaluate_type_decl<'t, 'b>(
    scope_node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error> {
    let name_parts = (
        qmatch.nodes_for_capture_index(*F_ID).next(),
        qmatch.nodes_for_capture_index(*F_GENERICS).next(),
    );
    let name = match name_parts {
        (Some(id), Some(generics)) => {
            QualifiedNameRef::from(str::from_utf8(&buffer[id.start_byte() .. generics.end_byte()]).map_err(|e| Error::Utf8(e))?)
        },
        (Some(id), None) => {
            QualifiedNameRef::try_from(id, buffer).map_err(|e| Error::BadName(e))?
        },
        (None, _) => {
            return Err(Error::FieldName("id"));
        },
    };

    result.type_decl.entry(scope_node).or_insert(HashSet::new())
        .insert(name);

    Ok(())
}

fn evaluate_type_usage<'t, 'b>(
    node: Node<'t>, _qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error> {
    // skip "using x = typename", included in ns_usage
    if let Some(user) = node.parent() && user.kind_id() == *K_USING {
        return Ok(());
    }

    let name = QualifiedNameRef::try_from(node, buffer).map_err(|e| Error::BadName(e))?;
    result.type_usages.insert(node, name);
    Ok(())
}

fn evaluate_var_decl<'t, 'b>(
    node: Node<'t>, qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error> {
    let id_node = match qmatch.nodes_for_capture_index(*F_ID).next() {
        Some(id) => id,
        None => return Err(Error::FieldName("id")),
    };
    let id = QualifiedNameRef::try_from(id_node, buffer).map_err(|e| Error::BadName(e))?;

    result.var_decl.entry(node)
        .or_insert(HashSet::new())
        .insert(id);
    Ok(())
}


fn evaluate_var_usage<'t, 'b>(
    node: Node<'t>, _qmatch: &QueryMatch<'_, 't>, buffer: &'b [u8], result: &mut StructureInfo<'b, 't>,
) -> Result<(), Error> {
    let name = QualifiedNameRef::try_from(node, buffer).map_err(|e| Error::BadName(e))?;
    result.var_usages.insert(node, name);
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        cmp::Eq,
        hash::Hash,
        fmt::Debug,
    };
    use crate::parser::csharp::test::{
        NS_TEST_CODE, 
        NS_TEST_TREE, 
        NodeLike, 
        TYPE_TEST_CODE,
        TYPE_TEST_TREE, 
        VAR_TEST_CODE, 
        VAR_TEST_TREE,
    };
    use super::*;

    const ROOT: NodeLike = NodeLike::new("compilation_unit", 0, 0);

    fn assert_map<'t, T, U, I>(
        actual: HashMap<Node<'t>, I>,
        expected: HashMap<T, I>,
    ) where T: From<Node<'t>> + PartialEq<Node<'t>> + Debug + Clone + Eq + Hash + Display,
        U: PartialEq<U> + Debug + Eq + Hash,
        I: IntoIterator<Item = U> + Clone {
        let mut matched = HashSet::new();
        let mut unexpected = HashSet::new();

        for (anode, aset) in actual.into_iter() {
            let eset = match expected.iter().find(|(nl, _)| **nl == anode) {
                Some((n, s)) => {
                    if !matched.insert(n.clone()) {
                        println!("Multiple matches for {n}");
                    }
                    s.clone()
                },
                None => {
                    unexpected.insert(T::from(anode));
                    continue;
                },
            };

            let aset = aset.into_iter().collect::<HashSet<U>>();
            let eset = eset.into_iter().collect::<HashSet<U>>();
            let unexpected: HashSet<&U> = aset.difference(&eset).collect();
            let missing: HashSet<&U> = eset.difference(&aset).collect();
            assert_eq!(unexpected, missing, "Mismatch between items under node {anode:?}");
        }

        let missing = expected.into_keys().filter(|n| !matched.contains(n)).collect::<HashSet<T>>();
        assert_eq!(unexpected, missing, "Mismatch between node lists");
    }

    #[test]
    fn evaluate_structure_ns() {
        let result = super::evaluate_structure(&NS_TEST_TREE, NS_TEST_CODE)
            .expect("Evaluation failed");

        assert_eq!(result.fsns_decl, Some(QualifiedNameRef::from("L0")));

        assert_map(result.ns_decl, HashMap::from([
            (ROOT.clone(), HashSet::from([
                QualifiedNameRef::from("L1"),
            ])),
            (NodeLike::new("declaration_list", 6, 0), HashSet::from([
                QualifiedNameRef::from("L2"),
            ])),
        ]));

        assert_map(result.ns_usages, HashMap::from([
            (ROOT.clone(), HashSet::from([
                QualifiedNameRef::from("Ns0"),
            ])),
            (NodeLike::new("declaration_list", 6, 0), HashSet::from([
                QualifiedNameRef::from("Ns1"),
            ])),
            (NodeLike::new("declaration_list", 12, 4), HashSet::from([
                QualifiedNameRef::from("Ns2"),
            ])),
        ]));

        assert_map(result.aliases, HashMap::from([
            (ROOT.clone(), HashMap::from([
                (QualifiedNameRef::from("InnerType"), QualifiedNameRef::from("Ns0")),
                (QualifiedNameRef::from("ns0a"), QualifiedNameRef::from("Ns0.InnerNs")),
            ])),
            (NodeLike::new("declaration_list", 6, 0), HashMap::from([
                (QualifiedNameRef::from("InnerType"), QualifiedNameRef::from("Ns1")),
                (QualifiedNameRef::from("ns1a"), QualifiedNameRef::from("Ns1.InnerNs")),
            ])),
            (NodeLike::new("declaration_list", 12, 4), HashMap::from([
                (QualifiedNameRef::from("InnerType"), QualifiedNameRef::from("Ns2")),
                (QualifiedNameRef::from("ns2a"), QualifiedNameRef::from("Ns2.InnerNs")),
            ])),
        ]));

        assert_eq!(result.type_decl, HashMap::new());
        assert_eq!(result.type_usages, HashMap::new());
        assert_eq!(result.var_decl, HashMap::new());
        assert_eq!(result.var_usages, HashMap::new());
    }

    #[test]
    fn evaluate_structure_type() {
        let result = super::evaluate_structure(&TYPE_TEST_TREE, TYPE_TEST_CODE)
            .expect("Evaluation failed");

        const NS1: NodeLike = NodeLike::new("declaration_list", 8, 0);

        assert_eq!(result.fsns_decl, Some(QualifiedNameRef::from("Ns0")));

        println!("Checking aliases");
        assert_map(result.aliases, HashMap::from([
            (NS1.clone(), HashMap::from([
                (QualifiedNameRef::from("ns3a"), QualifiedNameRef::from("Ns3")),
            ])),
        ]));

        println!("Checking ns_decl");
        assert_map(result.ns_decl, HashMap::from([
            (ROOT.clone(), HashSet::from([
                QualifiedNameRef::from("Ns1"),
                QualifiedNameRef::from("Ns3"),
            ])),
            (NS1.clone(), HashSet::from([
                QualifiedNameRef::from("Ns2"),
            ])),
        ]));

        println!("Checking type_decl");
        assert_map(result.type_decl, HashMap::from([
            (ROOT.clone(), HashSet::from([
                QualifiedNameRef::from("Enum0"),
            ])),
            (NS1.clone(), HashSet::from([
                QualifiedNameRef::from("Struct1<T>"),
                QualifiedNameRef::from("Class1"),
            ])),
            (NodeLike::new("declaration_list", 12, 4), HashSet::from([
                QualifiedNameRef::from("Record2"),
            ])),
            (NodeLike::new("struct_declaration", 16, 4), HashSet::from([
                QualifiedNameRef::from("T"),
            ])),
            (NodeLike::new("declaration_list", 22, 4), HashSet::from([
                QualifiedNameRef::from("ChildClass"),
            ])),
            (NodeLike::new("declaration_list", 36, 0), HashSet::from([
                QualifiedNameRef::from("INterface3"),
            ])),
        ]));

        println!("Checking type_usages");
        assert_map(result.type_usages, HashMap::from([
            (NodeLike::new("identifier", 18, 15), QualifiedNameRef::from("T")),
            (NodeLike::new("identifier", 25, 15), QualifiedNameRef::from("ChildClass")),
            (NodeLike::new("generic_name", 27, 15), QualifiedNameRef::from("Struct1<ns3a::INterface3>")),
            (NodeLike::new("alias_qualified_name", 27, 23), QualifiedNameRef::from("ns3a::INterface3")),
            (NodeLike::new("identifier", 29, 15), QualifiedNameRef::from("Enum0")),
            (NodeLike::new("qualified_name", 31, 15), QualifiedNameRef::from("Ns2.Record2")),
        ]));

        println!("Checking var_decl");
        assert_map(result.var_decl, HashMap::from([
            (NodeLike::new("declaration_list", 17, 4), HashSet::from([
                QualifiedNameRef::from("Value"),
            ])),
            (NodeLike::new("declaration_list", 22, 4), HashSet::from([
                QualifiedNameRef::from("ChildClassField"),
                QualifiedNameRef::from("SiblingStructProperty"),
                QualifiedNameRef::from("ParentEnumArray"),
                QualifiedNameRef::from("NieceRecordField"),
            ])),
        ]));

        assert_eq!(result.ns_usages, HashMap::new());
        assert_eq!(result.var_usages, HashMap::new());
    }

    #[test]
    fn evaluate_structure_var() {
        let result = super::evaluate_structure(&VAR_TEST_TREE, VAR_TEST_CODE)
            .expect("Failed to evaluate structure");

        println!("Testing aliases");
        assert_map(result.aliases, HashMap::from([
            (ROOT.clone(), HashMap::from([
                (QualifiedNameRef::from("X"), QualifiedNameRef::from("Ns1.Class2")),
            ])),
            (NodeLike::new("declaration_list", 3, 0), HashMap::from([
                (QualifiedNameRef::from("Y"), QualifiedNameRef::from("Ns1.Class3")),
            ])),
        ]));

        println!("Testing ns_decl");
        assert_map(result.ns_decl, HashMap::from([
            (ROOT.clone(), HashSet::from([
                QualifiedNameRef::from("Ns0"),
                QualifiedNameRef::from("Ns1"),
            ])),
        ]));

        println!("Testing ns_usages");
        assert_map(result.ns_usages, HashMap::from([
            (NodeLike::new("declaration_list", 3, 0), HashSet::from([
                QualifiedNameRef::from("System.Text"),
            ])),
        ]));

        println!("Testing type_decl");
        assert_map(result.type_decl, HashMap::from([
            (NodeLike::new("declaration_list", 3, 0), HashSet::from([
                QualifiedNameRef::from("Delegate1"),
                QualifiedNameRef::from("Class1"),
            ])),
            (NodeLike::new("declaration_list", 34, 0), HashSet::from([
                QualifiedNameRef::from("Class2"),
                QualifiedNameRef::from("Class3"),
            ])),
        ]));

        println!("Testing type_usages");
        assert_map(result.type_usages, HashMap::from([
            (NodeLike::new("identifier", 7, 20), QualifiedNameRef::from("X")),
            (NodeLike::new("identifier", 7, 32), QualifiedNameRef::from("X")),
            (NodeLike::new("identifier", 7, 41), QualifiedNameRef::from("Y")),

            (NodeLike::new("identifier", 11, 15), QualifiedNameRef::from("X")),
            (NodeLike::new("identifier", 13, 15), QualifiedNameRef::from("Y")),
            (NodeLike::new("identifier", 15, 21), QualifiedNameRef::from("Delegate1")),

            (NodeLike::new("identifier", 19, 12), QualifiedNameRef::from("X")),
            (NodeLike::new("identifier", 21, 19), QualifiedNameRef::from("StringBuilder")),
        ]));

        println!("Testing var_decl");
        assert_map(result.var_decl, HashMap::from([
            (NodeLike::new("declaration_list", 10, 4), HashSet::from([
                QualifiedNameRef::from("Field"),
                QualifiedNameRef::from("Property"),
                QualifiedNameRef::from("Delegate"),
                QualifiedNameRef::from("Repeat"),
            ])),
            (NodeLike::new("method_declaration", 17, 8), HashSet::from([
                QualifiedNameRef::from("count"),
            ])),
            (NodeLike::new("block", 18, 8), HashSet::from([
                QualifiedNameRef::from("x"),
                QualifiedNameRef::from("test"),
            ])),
            (NodeLike::new("using_statement", 20, 12), HashSet::from([
                QualifiedNameRef::from("test"),
            ])),
            (NodeLike::new("using_statement", 21, 12), HashSet::from([
                QualifiedNameRef::from("sb"),
            ])),
            (NodeLike::new("for_statement", 23, 16), HashSet::from([
                QualifiedNameRef::from("i"),
            ])),
        ]));

        println!("Testing var_usages");
        assert_map(result.var_usages, HashMap::from([
            (NodeLike::new("identifier", 19, 18), QualifiedNameRef::from("Delegate")),
            (NodeLike::new("identifier", 19, 35), QualifiedNameRef::from("Field")),
            (NodeLike::new("identifier", 19, 42), QualifiedNameRef::from("Property")),
            (NodeLike::new("identifier", 20, 30), QualifiedNameRef::from("FakeClass")),
            (NodeLike::new("identifier", 21, 38), QualifiedNameRef::from("Main")),
            (NodeLike::new("identifier", 23, 32), QualifiedNameRef::from("i")),
            (NodeLike::new("identifier", 23, 36), QualifiedNameRef::from("count")),
            (NodeLike::new("identifier", 23, 43), QualifiedNameRef::from("i")),
            (NodeLike::new("identifier", 25, 20), QualifiedNameRef::from("sb")),
            (NodeLike::new("identifier", 25, 30), QualifiedNameRef::from("x")),
            (NodeLike::new("identifier", 27, 23), QualifiedNameRef::from("sb")),
        ]));

        assert_eq!(result.fsns_decl, None);
    }
}
