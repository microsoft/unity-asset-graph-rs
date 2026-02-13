use const_format::{formatcp, concatcp};
use tree_sitter::Query;
use std::sync::LazyLock;
use super::CS_LANG;

/// Finds all the namespace declarations. Captures the containing scope "ns_decl" and the name "id".
const NS_DECL: &str = r#"
    (compilation_unit [
        (namespace_declaration
            name: [(qualified_name) (identifier)] @id
        )
        (file_scoped_namespace_declaration
            name: (_) @id
        )
    ]) @ns_decl
    (declaration_list
        (namespace_declaration
            name: [(qualified_name) (identifier)] @id
        )
    ) @ns_decl
"#;

/// Finds all the "normal" namespace import directives. Captures the directive "ns_use" and the namespace "id".
/// NOTE: Includes "using static" directives, which are not namespace imports. Must account for these manually.
const NS_USAGE: &str = r#"
    (compilation_unit [
        (using_directive
            name: (identifier) @alias
            [(qualified_name) (identifier)] @id
        )
        (using_directive
            [(qualified_name) (identifier)] @id
            !name
        )
    ]) @ns_use
    (declaration_list [
        (using_directive
            name: (identifier) @alias
            [(qualified_name) (identifier)] @id
        )
        (using_directive
            [(qualified_name) (identifier)] @id
            !name
        )
    ]) @ns_use
"#;

/// Find all type declarations, and capture the type ID "id"
const TYPE_DECL_ID: &str = r#"
    (class_declaration
        name: (identifier) @id
        (type_parameter_list)? @generics
    )
    (delegate_declaration
        name: (identifier) @id
        (type_parameter_list)? @generics
    )
    (enum_declaration
        name: (identifier) @id
        (type_parameter_list)? @generics
    )
    (interface_declaration
        name: (identifier) @id
        (type_parameter_list)? @generics
    )
    (record_declaration
        name: (identifier) @id
        (type_parameter_list)? @generics
    )
    (struct_declaration
        name: (identifier) @id
        (type_parameter_list)? @generics
    )
"#;

/// Find all type declarations, and capture the type ID "id"
const TYPE_DECL_GENERIC_ID: &str = r#"
    (class_declaration
        (type_parameter_list
            (type_parameter name: (identifier) @id)
        )
    ) @type_decl
    (delegate_declaration
        (type_parameter_list
            (type_parameter name: (identifier) @id)
        )
    ) @type_decl
    (interface_declaration
        (type_parameter_list
            (type_parameter name: (identifier) @id)
        )
    ) @type_decl
    (record_declaration
        (type_parameter_list
            (type_parameter name: (identifier) @id)
        )
    ) @type_decl
    (struct_declaration
        (type_parameter_list
            (type_parameter name: (identifier) @id)
        )
    ) @type_decl
"#;

/// Find all top-level type declarations, i.e. in a namespace or un-namespaced.
/// Captures the declaration scope "type_decl" and the type identifier "id".
const TYPE_DECL: &str = formatcp!(r#"
    (declaration_list
        [{TYPE_DECL_ID}]
    ) @type_decl
    (compilation_unit
        [{TYPE_DECL_ID}]
    ) @type_decl
    {TYPE_DECL_GENERIC_ID}
"#);

/// Matches all usages of types. Captures the type name "type_use", including any generic args.
/// Note: Includes types in `using alias = T` directives. These must be excluded manually.
const TYPE_USAGE: &str = r#"
    (type/identifier) @type_use
    (type/generic_name) @type_use
    (type/alias_qualified_name) @type_use
    (type/qualified_name
        qualifier: [(identifier) (qualified_name) (generic_name)]
    ) @type_use
    (type/tuple_type
        (tuple_element
            type: [(identifier) (qualified_name) (generic_name)] @type_use
        )
    )
    (type/scoped_type
        type: [(identifier) (qualified_name) (generic_name)] @type_use
    )
    (type/array_type 
        type: [(identifier) (qualified_name) (generic_name)] @type_use
    )
    (type/nullable_type 
        type: [(identifier) (qualified_name) (generic_name)] @type_use
    )
    (type/ref_type 
        type: [(identifier) (qualified_name) (generic_name)] @type_use
    )
"#;


/// Matches a variable declaration. Captures the var identifier "id"
const VAR_DECL_ID: &str = r#"
    (variable_declaration
        (variable_declarator
            name: (identifier) @id
        )
    )
"#;

/// Matches a function argument declaration. Captures the param identifier "id"
const PARAM_DECL_ID: &str = r#"
    (parameter_list
        (parameter
            name: (identifier) @id
        )
    )
"#;

const VAR_DECL_PARTS: [&str; 4] = [
    // "normal" variables in a code block
    formatcp!(r#"
        (block [
            (local_declaration_statement {VAR_DECL_ID})
            (fixed_statement {VAR_DECL_ID})
            (using_statement
                {VAR_DECL_ID}
                body: (empty_statement)
            )
        ])
    "#),

    // declared variable in a special statement
    formatcp!(r#"
        (for_statement initializer: {VAR_DECL_ID})
        (using_statement
            {VAR_DECL_ID}
        )
    "#),

    // identifiers declared in a namespace or type body, i.e. type/field/property/method names
    formatcp!(r#"
        (declaration_list [
            (field_declaration {VAR_DECL_ID})
            (event_field_declaration {VAR_DECL_ID})
            (property_declaration name: (identifier) @id)
            (method_declaration name: (identifier) @id)
        ])
    "#),

    // variables declared as function arguments
    formatcp!(r#"
        (constructor_declaration parameters: {PARAM_DECL_ID})
        (method_declaration parameters: {PARAM_DECL_ID})
        (operator_declaration parameters: {PARAM_DECL_ID})
        (lambda_expression parameters: {PARAM_DECL_ID})
        (anonymous_method_expression parameters: {PARAM_DECL_ID})
        (local_function_statement parameters: {PARAM_DECL_ID})
        (indexer_declaration
            parameters: (bracketed_parameter_list
                (parameter
                    name: (identifier) @id
                )
            )
        )
    "#),
];

/// Matches all declarations of all types of variables. Captures the variable identifier "id" and its scope "var_decl"
const VAR_DECL: &str = concatcp!(
    "[", VAR_DECL_PARTS[0], VAR_DECL_PARTS[1], VAR_DECL_PARTS[2], VAR_DECL_PARTS[3], "] @var_decl"
);

/// Matches all uses of variables, which will include some type references not caught by `TYPE_USAGE`. Filter against
/// `VAR_DECL` in the current scope to find them. Captures the variable/type name as "var_use".
const VAR_USAGE: &str = r#"
    (expression/identifier) @var_use
    (expression/generic_name) @var_use
"#;

/// Matches everything we're looking for. Captures "ns_use", "type_decl", "var_decl", "id", "type_use", and "var_use".
pub const QUERY_ALL: &str = concatcp!(
    NS_DECL,
    NS_USAGE,
    TYPE_DECL,
    TYPE_USAGE,
    VAR_DECL,
    VAR_USAGE,
);

pub static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&CS_LANG, QUERY_ALL).expect("Failed to compile query")
});

pub mod fields {
    use super::*;
    pub static NS_DECL: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("ns_decl").expect("Failed to get field ns_decl"));
    pub static NS_USE: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("ns_use").expect("Failed to get field ns_use"));
    pub static TYPE_DECL: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("type_decl").expect("Failed to get field type_decl"));
    pub static TYPE_USE: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("type_use").expect("Failed to get field type_use"));
    pub static VAR_DECL: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("var_decl").expect("Failed to get field var_decl"));
    pub static VAR_USE: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("var_use").expect("Failed to get field var_use"));
    pub static ID: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("id").expect("Failed to get field id"));
    pub static ALIAS: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("alias").expect("Failed to get field alias"));
    pub static GENERICS: LazyLock<u32> = LazyLock::new(|| QUERY.capture_index_for_name("generics").expect("Failed to get field generics"));
}

pub mod kinds {
    use super::*;
    pub static FILE_SCOPED_NS_DECL: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("file_scoped_namespace_declaration", true));
    pub static USING: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("using_directive", true));
    pub static STATIC: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("static", false));
    pub static MEMBER_ACCESS_EXPR: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("member_access_expression", true));
    pub static INVOCATION_EXPR: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("invocation_expression", true));
    pub static ELEMENT_ACCESS_EXPR: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("element_access_expression", true));
    pub static GENERIC_NAME: LazyLock<u16> = LazyLock::new(|| CS_LANG.id_for_node_kind("generic_name", true));
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use tree_sitter::{Query, QueryCursor, QueryError, QueryMatch, StreamingIterator};
    //use pretty_assertions::assert_eq;
    use crate::parser::csharp::{
        CS_LANG,
        test::{
            NS_TEST_CODE, NS_TEST_TREE, NodeLike, TYPE_TEST_CODE, TYPE_TEST_TREE, VAR_TEST_CODE, VAR_TEST_TREE
        },
    };

    fn assert_matches<'c, 't>(
        query: &Query,
        mut actual: impl StreamingIterator<Item = QueryMatch<'c, 't>>,
        expected: Vec<HashMap<&str, NodeLike>>,
    ) where 't: 'c {
        let mut matched = HashSet::new();
        let mut unexpected = vec![];

        // build a mapping of capture names in the expected set to capture indexes in the query
        let mut capture_ids = HashMap::new();
        for ecaps in &expected {
            for cap_name in ecaps.keys() {
                let id = query.capture_index_for_name(cap_name)
                    .expect(&format!("Failed to get capture index for name '{cap_name}'"));
                capture_ids.entry(id).or_insert(*cap_name);
            }
        }

        while let Some(amatch) = actual.next() {
            // find the expected match
            let exp = expected.iter().enumerate().find(|(_, ecaps)| {
                // all of whose captures
                amatch.captures.iter().all(|c| {
                    // are in the name map
                    let capture_name = match capture_ids.get(&c.index).map(|n| *n) {
                        Some(n) => n,
                        None => return false,
                    };
                    // and whose captured nodes match
                    match ecaps.get(capture_name) {
                        Some(enode) => *enode == c.node,
                        None => false,
                    }
                })
            });

            if let Some((i, _)) = exp {
                if !matched.insert(i) {
                    println!("Multiply-matched: {:?}", expected[i]);
                }
            } else {
                let captures: HashMap<&str, NodeLike> = amatch.captures.iter()
                    .map(|c| (
                        capture_ids.get(&c.index).map(|n| *n).unwrap(),
                        NodeLike::from(c.node),
                    ))
                    .collect();
                unexpected.push(captures);
            }
        }

        let unmatched: Vec<HashMap<&str, NodeLike>> = expected.iter().enumerate()
            .filter_map(|(i, m)| {
                if !matched.contains(&i) { Some(m.clone()) } else { None }
            })
            .collect();
        assert_eq!(unexpected, unmatched);
    }

    #[test]
    fn ns_decl() {
        let query = Query::new(&CS_LANG, NS_DECL).expect("Failed to compile namespace query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, NS_TEST_TREE.root_node(), NS_TEST_CODE);
        assert_matches(&query, iter, vec![
            HashMap::from([
                ("ns_decl", NodeLike::new("compilation_unit", 0, 0)),
                ("id", NodeLike::new("identifier", 0, 10)),
            ]),
            // namespace L1
            HashMap::from([
                ("ns_decl", NodeLike::new("compilation_unit", 0, 0)),
                ("id", NodeLike::new("identifier", 5, 10)),
            ]),
            // namespace L2
            HashMap::from([
                ("ns_decl", NodeLike::new("declaration_list", 6, 0)),
                ("id", NodeLike::new("identifier", 11, 14)),
            ]),
            // namespace L3
            HashMap::from([
                ("ns_decl", NodeLike::new("declaration_list", 6, 0)),
                ("id", NodeLike::new("identifier", 20, 14)),
            ]),
        ]);
    }

    #[test]
    fn ns_usage() {
        let query = Query::new(&CS_LANG, NS_USAGE).expect("Failed to compile namespace query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, NS_TEST_TREE.root_node(), NS_TEST_CODE);
        assert_matches(&query, iter, vec![
            // using Ns0
            HashMap::from([
                ("ns_use", NodeLike::new("compilation_unit", 0, 0)),
                ("id", NodeLike::new("identifier", 1, 6)),
            ]),
            // using ns0a = Ns0.InnerNs
            HashMap::from([
                ("ns_use", NodeLike::new("compilation_unit", 0, 0)),
                ("alias", NodeLike::new("identifier", 2, 6)),
                ("id", NodeLike::new("qualified_name", 2, 13)),
            ]),
            // using static Ns0.InnerType
            HashMap::from([
                ("ns_use", NodeLike::new("compilation_unit", 0, 0)),
                ("id", NodeLike::new("qualified_name", 3, 13)),
            ]),
            // using Ns1
            HashMap::from([
                ("ns_use", NodeLike::new("declaration_list", 6, 0)),
                ("id", NodeLike::new("identifier", 7, 10)),
            ]),
            // using ns1a = Ns1.InnerNs
            HashMap::from([
                ("ns_use", NodeLike::new("declaration_list", 6, 0)),
                ("alias", NodeLike::new("identifier", 8, 10)),
                ("id", NodeLike::new("qualified_name", 8, 17)),
            ]),
            // using static Ns1.InnerType
            HashMap::from([
                ("ns_use", NodeLike::new("declaration_list", 6, 0)),
                ("id", NodeLike::new("qualified_name", 9, 17)),
            ]),
            // using Ns2
            HashMap::from([
                ("ns_use", NodeLike::new("declaration_list", 12, 4)),
                ("id", NodeLike::new("identifier", 13, 14)),
            ]),
            // using ns2a = Ns2.InnerNs
            HashMap::from([
                ("ns_use", NodeLike::new("declaration_list", 12, 4)),
                ("alias", NodeLike::new("identifier", 14, 14)),
                ("id", NodeLike::new("qualified_name", 14, 21)),
            ]),
            // using static Ns2.InnerType
            HashMap::from([
                ("ns_use", NodeLike::new("declaration_list", 12, 4)),
                ("id", NodeLike::new("qualified_name", 15, 21)),
            ]),
        ]);
    }

    #[test]
    fn type_decl() {
        let query = Query::new(&CS_LANG, TYPE_DECL).expect("Failed to compile namespace query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, TYPE_TEST_TREE.root_node(), TYPE_TEST_CODE);
        assert_matches(&query, iter, vec![
            // Enum0
            HashMap::from([
                ("type_decl", NodeLike::new("compilation_unit", 0, 0)),
                ("id", NodeLike::new("identifier", 2, 12)),
            ]),
            // Record2
            HashMap::from([
                ("type_decl", NodeLike::new("declaration_list", 12, 4)),
                ("id", NodeLike::new("identifier", 13, 22)),
            ]),
            // Struct1
            HashMap::from([
                ("type_decl", NodeLike::new("declaration_list", 8, 0)),
                ("id", NodeLike::new("identifier", 16, 18)),
                ("generics", NodeLike::new("type_parameter_list", 16, 25))
            ]),
            // T
            HashMap::from([
                ("type_decl", NodeLike::new("struct_declaration", 16, 4)),
                ("id", NodeLike::new("identifier", 16, 26)),
            ]),
            // Class1
            HashMap::from([
                ("type_decl", NodeLike::new("declaration_list", 8, 0)),
                ("id", NodeLike::new("identifier", 21, 17)),
            ]),
            // ChildClass
            HashMap::from([
                ("type_decl", NodeLike::new("declaration_list", 22, 4)),
                ("id", NodeLike::new("identifier", 23, 21)),
            ]),
            // INterface3
            HashMap::from([
                ("type_decl", NodeLike::new("declaration_list", 36, 0)),
                ("id", NodeLike::new("identifier", 37, 21)),
            ]),
        ]);
    }

    #[test]
    fn type_usage() {
        let query = Query::new(&CS_LANG, TYPE_USAGE).expect("Failed to compile namespace query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, TYPE_TEST_TREE.root_node(), TYPE_TEST_CODE);
        assert_matches(&query, iter, vec![
            // using ns3a
            HashMap::from([("type_use", NodeLike::new("identifier", 9, 17))]),
            // T Value
            HashMap::from([("type_use", NodeLike::new("identifier", 18, 15))]),
            // ChildClassField
            HashMap::from([("type_use", NodeLike::new("identifier", 25, 15))]),
            // SiblingStructProperty
            HashMap::from([("type_use", NodeLike::new("generic_name", 27, 15))]),
            // SiblingStructProperty generic
            HashMap::from([("type_use", NodeLike::new("alias_qualified_name", 27, 23))]),
            // ParentEnumArray
            HashMap::from([("type_use", NodeLike::new("identifier", 29, 15))]),
            // NieceRecordField
            HashMap::from([("type_use", NodeLike::new("qualified_name", 31, 15))]),
        ]);
    }

    #[test]
    fn var_decl() {
        let query = Query::new(&CS_LANG, VAR_DECL).expect("Failed to compile namespace query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, VAR_TEST_TREE.root_node(), VAR_TEST_CODE);
        assert_matches(&query, iter, vec![
            // Field
            HashMap::from([
                ("var_decl", NodeLike::new("declaration_list", 10, 4)),
                ("id", NodeLike::new("identifier", 11, 17)),
            ]),
            // Property
            HashMap::from([
                ("var_decl", NodeLike::new("declaration_list", 10, 4)),
                ("id", NodeLike::new("identifier", 13, 17)),
            ]),
            // Delegate
            HashMap::from([
                ("var_decl", NodeLike::new("declaration_list", 10, 4)),
                ("id", NodeLike::new("identifier", 15, 31)),
            ]),
            // Repeat
            HashMap::from([
                ("var_decl", NodeLike::new("declaration_list", 10, 4)),
                ("id", NodeLike::new("identifier", 17, 22)),
            ]),
            // count
            HashMap::from([
                ("var_decl", NodeLike::new("method_declaration", 17, 8)),
                ("id", NodeLike::new("identifier", 17, 33)),
            ]),
            // x
            HashMap::from([
                ("var_decl", NodeLike::new("block", 18, 8)),
                ("id", NodeLike::new("identifier", 19, 14)),
            ]),
            // test (block)
            HashMap::from([
                ("var_decl", NodeLike::new("block", 18, 8)),
                ("id", NodeLike::new("identifier", 20, 23)),
            ]),
            // test (local)
            HashMap::from([
                ("var_decl", NodeLike::new("using_statement", 20, 12)),
                ("id", NodeLike::new("identifier", 20, 23)),
            ]),
            // sb
            HashMap::from([
                ("var_decl", NodeLike::new("using_statement", 21, 12)),
                ("id", NodeLike::new("identifier", 21, 33)),
            ]),
            // i
            HashMap::from([
                ("var_decl", NodeLike::new("for_statement", 23, 16)),
                ("id", NodeLike::new("identifier", 23, 25)),
            ]),
        ]);
    }

    #[test]
    fn var_usage() {
        let query = Query::new(&CS_LANG, VAR_USAGE).expect("Failed to compile namespace query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, VAR_TEST_TREE.root_node(), VAR_TEST_CODE);
        assert_matches(&query, iter, vec![
            // Delegate
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 19, 18)),
            ]),
            // Field
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 19, 35)),
            ]),
            // Property
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 19, 42)),
            ]),
            // FakeClass (missing)
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 20, 30)),
            ]),
            // StringBuilderCache (missing)
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 21, 38)),
            ]),
            // i (test)
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 23, 32)),
            ]),
            // count (test)
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 23, 36)),
            ]),
            // i (increment)
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 23, 43)),
            ]),
            // sb append (missing)
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 25, 20)),
            ]),
            // x append
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 25, 30)),
            ]),
            // sb return (missing)
            HashMap::from([
                ("var_use", NodeLike::new("identifier", 27, 23)),
            ]),
        ]);
    }

    #[test]
    fn query_all() -> Result<(), QueryError> {
        Query::new(&CS_LANG, QUERY_ALL).map(|_| ())
    }
}
