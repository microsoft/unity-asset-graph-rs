use crate::{Asset, Id, Relation, parser::ParseError};
use ansi_term::Color::Yellow;
use std::{path::Path, sync::LazyLock};
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

static LOCSTR_QUERY: LazyLock<Query> = LazyLock::new(|| {
    match Query::new(
        &super::CS_LANG,
        r#"
(invocation_expression
    function: (member_access_expression
        expression: (
            (identifier) @obj-name
            (#eq? @obj-name "LocStringCache")
        )
        name: (
            (identifier) @fn-name
            (#eq? @fn-name "Get")
        )
    )
    arguments: (argument_list
        [
            (argument
                .
                (string_literal) @loc-str
            )
            (argument
                ((identifier) @arg-name (#eq? @arg-name "key"))
                .
                (string_literal) @loc-str
            )
        ]
    )
)"#,
    ) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Failed to compile locstring query: {e}");
            panic!();
        }
    }
});

pub fn find_locstrings(tree: &Tree, buffer: &[u8], path: &Path, asset: &mut Asset) -> Result<(), ParseError> {
    // loop over all locstring cache gets
    let mut q = QueryCursor::new();
    let mut iter = q.matches(&LOCSTR_QUERY, tree.root_node(), buffer);

    while let Some(m) = iter.next() {
        let literal_match = m
            .captures
            .iter()
            .find(|c| c.index == LOCSTR_QUERY.capture_index_for_name("loc-str").unwrap());
        let node = literal_match.unwrap().node;

        if node.kind() == "string_literal" {
            // trim open/close quotes
            let text = match node.utf8_text(buffer) {
                Ok(t) => &t[1..t.len() - 1],
                Err(_) => {
                    eprintln!("\nFailed to read UTF-8 from {}", path.display());
                    continue;
                }
            };
            asset.relations.insert(Relation::Uses(Id::Loc(text.into())));
        } else {
            let pos = node.start_position();
            let text = match node.utf8_text(buffer) {
                Ok(t) => t,
                Err(_) => {
                    eprintln!("\nFailed to read UTF-8 from {}", path.display());
                    continue;
                }
            };
            eprintln!(
                "\n{}: Failed to index non-literal localized string key '{text}' ({}) ({}, line {} col {})",
                Yellow.paint("Warning"),
                node.kind(),
                path.display(),
                pos.row + 1,
                pos.column + 1
            );
            continue;
        }
    }
    Ok(())
}
