
use tree_sitter::{
    Node, 
    Query as TSQuery, 
    QueryCursor, 
    StreamingIterator, 
    Tree,
};

pub trait UsingQuery {
    fn alias_node(&self) -> Node;
    fn type_node(&self) -> Node;
}

pub struct Query {
    src: &'static str,
    q: Option<TSQuery>,
}

impl Query {
    const fn new(src: &'static str) -> Self {
        Self {
            src,
            q: None,
        }
    }

    const fn new_using() -> Self {
        Self {
            src: r#"
            (using_directive
                name: (identifier)? @alias
                (type) @type
            )"#,
            q: None,
        }
    }

    pub fn q(&mut self) -> &TSQuery {
        if self.q.is_none() {
            self.q = Some(TSQuery::new(&super::CS_LANG, self.src).expect("Failed to compile query"));
        }
        self.q.as_ref().unwrap()
    }

}
