use tree_sitter::{
    Node,
    Query,
    QueryCursor, 
    QueryError,
    StreamingIterator, 
    Tree,
};
use std::{
    collections::HashSet, fmt::{Display, Formatter, Result as FResult}, sync::LazyLock
};
use const_format::formatcp;
use crate::parser::csharp::CS_LANG;

const USE_NS: &str = r#"
    (using_directive
        [(identifier) (qualified_name)] @ns
        !name
    )
"#;

const QUERY_ALL: &str = formatcp!(r#"
    [
        {USE_NS}
    ]
"#);

#[derive(Debug)]
pub enum Error<'a> {
    Query(QueryError),
    Field(&'a str),
    Utf8,
}
impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        match self {
            Self::Query(q) => write!(f, "{q}"),
            Self::Field(e) => write!(f, "No such field '{e}'"),
            Self::Utf8 => write!(f, "Failed to convert buffer to UTF-8"),
        }
    }
}
impl<'a> std::error::Error for Error<'a> {}

fn get_query_output<'a, 'b>(q_text: &'a str, field: &'a str, node: Node<'b>, buffer: &'b [u8]) -> Result<HashSet<&'b str>, Error<'a>> {
    let query = Query::new(&CS_LANG, q_text).map_err(|e| Error::Query(e))?;
    let field = match query.capture_index_for_name(field) {
        Some(i) => i,
        _ => return Err(Error::Field(field)),
    };

    let mut cursor = QueryCursor::new();
    let mut iter = cursor.matches(&query, node, buffer);
    
    let mut results = HashSet::new();
    while let Some(m) = iter.next() {
        let id = m.captures[field as usize].node.utf8_text(buffer).map_err(|_| Error::Utf8)?;
        results.insert(id);
    }
    Ok(results)
}

#[cfg(test)]
mod test {
    use tree_sitter::Parser;

    use super::*;

    const CODE: &[u8] = include_bytes!("../csharp_test.cs");
    static TREE: LazyLock<Tree> = LazyLock::new(|| {
        let mut parser = Parser::new();
        parser.set_language(&CS_LANG).expect("Failed to load C# language");
        parser.parse(CODE, None).expect("Failed to parse test file")
    });

    #[test]
    fn use_ns() -> Result<(), Error<'static>> {
        let namespaces = get_query_output(USE_NS, "ns", TREE.root_node(), CODE)?;
        assert_eq!(namespaces, HashSet::from(["System", "My.DifferentNamespace"]));
        Ok(())
    }
}

pub static USING_QUERY: LazyLock<Query> = LazyLock::new(|| {
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
        ]
    "#).expect("Failed to compile using query")
});

/// Query to find class, struct, enum, and interface declarations.
/// Syntax tree identifiers come from https://github.com/tree-sitter/tree-sitter-c-sharp/blob/master/src/node-types.json
pub static TYPE_DECL: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
        (type_declaration) @decl
    "#).expect("Failed to compile class query")
});

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

pub static VAR_DECL: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
        [
            (block
                ([(local_declaration_statement) (fixed_statement) (using_statement)]
                    (variable_declaration
                        (variable_declarator
                            name: (identifier) @varname
                        )
                    )
                )
            )
            (for_statement
                initializer: (variable_declaration
                    (variable_declarator
                        name: (identifier) @varname
                    )
                )
            )
            (type_declaration
                name: (identifier) @varname
            )
            (type_declaration
                body: (declaration_list
                    ([(field_declaration) (event_field_declaration)]
                        (variable_declaration
                            (variable_declarator
                                name: (identifier) @varname
                            )
                        )
                    )
                )
            )
            (type_declaration
                body: (declaration_list
                    (declaration
                        name: (identifier) @varname
                    )
                )
            )
            ([(declaration) (lambda_expression) (anonymous_method_expression) (local_function_statement)]
                ([(parameter_list) (bracketed_parameter_list)]
                    (parameter
                        name: (identifier) @varname
                    )
                )
            )
        ] @scope
    "#).expect("Failed to compile variable usage query")
});

pub static VAR_USAGE: LazyLock<Query> = LazyLock::new(|| {
    Query::new(&super::CS_LANG, r#"
        (member_access_expression
            expression: [(identifier) (generic_name) (qualified_name)] @name
        )
    "#).expect("Failed to compile variable usage query")
});