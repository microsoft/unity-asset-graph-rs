use tree_sitter::{
    Node, Query, QueryCursor, QueryError, QueryMatch, StreamingIterator, Tree
};
use std::{
    collections::HashSet, fmt::{Display, Formatter, Result as FResult}, sync::LazyLock
};
use const_format::{formatcp, concatcp};
use super::CS_LANG;

#[derive(Debug)]
pub enum Error<'a> {
    Query(QueryError),
    FieldName(&'a str),
    FieldId(u32),
    Utf8,
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        match self {
            Self::Query(q) => write!(f, "{q}"),
            Self::FieldName(e) => write!(f, "No such field '{e}'"),
            Self::FieldId(id) => write!(f, "No such field {id}"),
            Self::Utf8 => write!(f, "Failed to convert buffer to UTF-8"),
        }
    }
}

impl<'a> std::error::Error for Error<'a> {}

fn collect_set<'c, 't>(
    mut iter: impl StreamingIterator<Item = QueryMatch<'c, 't>>, 
    field: u32, 
    buffer: &'_ [u8],
) -> Result<HashSet<&'_ str>, Error<'_>>
where 't: 'c {
    let mut results = HashSet::new();
    while let Some(m) = iter.next() {
        let node = match m.nodes_for_capture_index(field).next() {
            Some(n) => n,
            _ => return Err(Error::FieldId(field)),
        };
        let id = node.utf8_text(buffer).map_err(|_| Error::Utf8)?;
        results.insert(id);
    }
    Ok(results)
}

fn _debug_up(node: Node, buffer: &[u8]) {
    let mut n = Some(node);
    while let Some(node) = n {
        let text = node.utf8_text(buffer).unwrap().split('\n').next().unwrap();
        if text.len() < 100 {
            println!("{}: {}", node.kind(), text);
        }
        else {
            println!("{}: {}...<{} bytes>", node.kind(), &text[..100], node.end_byte() - node.start_byte() - 100);
        }
        n = node.parent();
    }
    println!();
}

fn _debug_down(node: Node, buffer: &[u8], max_depth: usize) {
    fn helper(node: Node, buffer: &[u8], depth: usize, max_depth: usize) {
        let indent = " ".repeat(depth);
        let kind = node.kind();
        let text = node.utf8_text(buffer).unwrap().split('\n').next().unwrap();
        if text.len() < 100 {
            println!("{indent}{kind}: {text}");
        }
        else {
            println!("{indent}{kind}: {}...<{} bytes>", &text[..100], node.end_byte() - node.start_byte() - 100);
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

const USE_NS: &str = r#"
    (using_directive
        [(qualified_name) (identifier)] @id
        !name
    ) @use_ns
"#;


fn filter_ns(query: &Query, m: &QueryMatch) -> bool {
    if let Some(ns) = query.capture_index_for_name("use_ns")
        && let Some(id) = query.capture_index_for_name("id")
        && let Some(_) = m.nodes_for_capture_index(ns).next()
        && let Some(id_node) = m.nodes_for_capture_index(id).next() {

        // cannot filter out "using static" directly from the query, do it here
        if let Some(prev) = id_node.prev_sibling() && prev.kind() == "static" {
            false
        } else {
            true
        }
    } else {
        false
    }
}

const TYPE_DECL_ID: &str = r#"
    (class_declaration
        name: (identifier) @id
    )
    (delegate_declaration
        name: (identifier) @id
    )
    (enum_declaration
        name: (identifier) @id
    )
    (interface_declaration
        name: (identifier) @id
    )
    (record_declaration
        name: (identifier) @id
    )
    (struct_declaration
        name: (identifier) @id
    )
"#;

const TYPE_DECL: &str = formatcp!(r#"
    [
        (namespace_declaration
            body: (declaration_list
                [{TYPE_DECL_ID}] @type_decl
            )
        )
        (compilation_unit
            [{TYPE_DECL_ID}] @type_decl
        )
    ]
"#);

const VAR_DECL_ID: &str = r#"
    (variable_declaration
        (variable_declarator
            name: (identifier) @id
        )
    )
"#;

const PARAM_DECL_ID: &str = r#"
[
    (parameter_list
        (parameter
            name: (identifier) @id
        )
    )
    (bracketed_parameter_list
        (parameter
            name: (identifier) @id
        )
    )
]
"#;

const VAR_DECL: &str = concatcp!(
"[",
    // "normal" variables in a code block
    formatcp!(r#"(block [
        (local_declaration_statement
            {VAR_DECL_ID}
        )
        (fixed_statement
            {VAR_DECL_ID}
        )
        (using_statement
            {VAR_DECL_ID}
        )
    ])"#),
    // the iterator in a for statement
    formatcp!(r#"(for_statement
        initializer: {VAR_DECL_ID}
    )"#),
    // un-namespaced type declarations
    formatcp!(r#"(compilation_unit
        [{TYPE_DECL_ID}]
    )"#),
    // identifiers declared in a namespace or type body, i.e. field/property/method names
    formatcp!(r#"(declaration_list [
        [{TYPE_DECL_ID}]
        (field_declaration {VAR_DECL_ID})
        (event_field_declaration {VAR_DECL_ID})
        (property_declaration)
        (method_declaration)

    ])"#),
    // variables declared as function arguments
    formatcp!(r#"[
        (constructor_declaration {PARAM_DECL_ID})
        (indexer_declaration {PARAM_DECL_ID})
        (method_declaration {PARAM_DECL_ID})
        (operator_declaration {PARAM_DECL_ID})
        (lambda_expression {PARAM_DECL_ID})
        (anonymous_method_expression {PARAM_DECL_ID}
        (local_function_statement {PARAM_DECL_ID}
        (declaration) (lambda_expression) (anonymous_method_expression) (local_function_statement)]
        {PARAM_DECL_ID}
    ]"#),
"] @var_decl"
);

#[cfg(test)]
mod test {
    use tree_sitter::Parser;
    use super::*;

    const CODE: &[u8] = include_bytes!("../csharp_test.cs");
    static TREE: LazyLock<Tree> = LazyLock::new(|| {
        let mut parser = Parser::new();
        parser.set_language(&CS_LANG).expect("Failed to set language, bad lang version");
        parser.parse(CODE, None).expect("Failed to read code")
    });

    #[test]
    fn use_ns() -> Result<(), Error<'static>> {
        let query = Query::new(&CS_LANG, USE_NS).expect("Failed to compile namespace query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, TREE.root_node(), CODE)
            .filter(|m| filter_ns(&query, m));

        let field = match query.capture_index_for_name("id") {
            Some(f) => f,
            None => return Err(Error::FieldName("id")),
        };
        let namespaces = collect_set(iter, field, CODE)?;

        assert_eq!(namespaces, HashSet::from(["X"]));
        Ok(())
    }

    #[test]
    fn type_decl() -> Result<(), Error<'static>> {
        let query = Query::new(&CS_LANG, TYPE_DECL).expect("Failed to compile type declaration query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, TREE.root_node(), CODE);

        let field = match query.capture_index_for_name("id") {
            Some(f) => f,
            None => return Err(Error::FieldName("id")),
        };
        let namespaces = collect_set(iter, field, CODE)?;

        assert_eq!(namespaces, HashSet::from(["ClassB", "ClassC"]));
        Ok(())
    }

    #[test]
    fn var_decl() -> Result<(), Error<'static>> {
        let query = Query::new(&CS_LANG, VAR_DECL).expect("Failed to compile variable decl query");
        let mut cursor = QueryCursor::new();
        let iter = cursor.matches(&query, TREE.root_node(), CODE);

        let field = match query.capture_index_for_name("id") {
            Some(f) => f,
            None => return Err(Error::FieldName("id")),
        };
        let namespaces = collect_set(iter, field, CODE)?;

        assert_eq!(namespaces, HashSet::from(["ClassB", "ClassC"]));
        Ok(())
    }
}

pub static TYPE_USAGE: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
        [
            (type/identifier) @type
            (type/generic_name) @type
            (type/qualified_name
                qualifier: [(identifier) (qualified_name) (generic_name)]
            ) @type
            (type/tuple_type
                (tuple_element
                    type: [(identifier) (qualified_name) (generic_name)] @type
                )
            )
            (type/scoped_type
                type: [(identifier) (qualified_name) (generic_name)] @type
            )
            (type/array_type 
                type: [(identifier) (qualified_name) (generic_name)] @type
            )
            (type/nullable_type 
                type: [(identifier) (qualified_name) (generic_name)] @type
            )
            (type/ref_type 
                type: [(identifier) (qualified_name) (generic_name)] @type
            )
        ]
    "#).expect("Failed to compile usage query")
});

pub static VAR_USAGE: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
        (member_access_expression
            expression: [(identifier) (generic_name) (qualified_name)] @name
        )
    "#).expect("Failed to compile variable usage query")
});