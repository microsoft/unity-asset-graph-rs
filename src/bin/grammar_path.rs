use std::{
    collections::{HashMap, HashSet},
    error::Error, hash::RandomState,
};
use tree_sitter_c_sharp::NODE_TYPES;

#[derive(serde::Deserialize)]
struct Type {
    r#type: String,
    named: bool,
    #[serde(default = "Vec::new")]
    subtypes: Vec<TypeRef>,
    #[serde(default = "HashMap::new")]
    fields: HashMap<String, Field>,
    children: Option<Field>,
}

#[derive(serde::Deserialize)]
struct TypeRef {
    r#type: String,
    named: bool,
}

#[derive(serde::Deserialize)]
struct Field {
    multiple: bool,
    required: bool,
    #[serde(default = "Vec::new")]
    types: Vec<TypeRef>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let types: Vec<Type> = serde_json::from_str(NODE_TYPES)?;
    let types: HashMap<String, Type, RandomState> = HashMap::from_iter(types.into_iter().map(|t| (t.r#type.clone(), t)));

    let mut backmap = HashMap::new();
    for t in types.values() {
        for tr in &t.subtypes {
            backmap.entry(&tr.r#type).or_insert(HashSet::new()).insert(&t.r#type);
        }
        for f in t.fields.values() {
            for tr in &f.types {
                backmap.entry(&tr.r#type).or_insert(HashSet::new()).insert(&t.r#type);
            }
        }
        if let Some(children) = &t.children {
            for tr in &children.types {
                backmap.entry(&tr.r#type).or_insert(HashSet::new()).insert(&t.r#type);
            }
        }
    }

    let paths = find_back_path::<40>(
        &["variable_declarator", "parameter"],
        &[
            "declaration", "type_declaration", 
            "preproc_if", "preproc_elif", "preproc_else", 
            "anonymous_method_expression", "lambda_expression", "local_function_statement",
        ],
        &types, 
        &backmap);

    for p in &paths {
        println!("{:?}", p);
    }
    Ok(())
}

fn find_back_path<'a, 't, const T: usize>(
    node_types: &'a [&'a str],
    terminals: &'a [&'a str],
    types: &'t HashMap<String, Type, RandomState>,
    backmap: &'t HashMap<&'t String, HashSet<&'t String>>,
) -> Vec<Vec<&'t String>> {
    let mut path = Vec::with_capacity(T);
    let mut paths = vec![];
    let terminals: HashSet<&String> = terminals.iter().map(|s| &types.get(*s).unwrap().r#type).collect();

    for t in node_types {
        let r#type = &types.get(*t).unwrap().r#type;
        helper::<T>(&r#type, &mut path, &terminals, &types, &backmap, &mut paths);
    }

    fn helper<'a, 't, const T: usize>(
        node_type: &'t String,
        path: &'a mut Vec<&'t String>,
        terminals: &'a HashSet<&'t String>,
        types: &'t HashMap<String, Type, RandomState>,
        backmap: &'t HashMap<&'t String, HashSet<&'t String>>,
        out: &'a mut Vec<Vec<&'t String>>,
    ) {
        // do not create cyclic or long paths
        if path.contains(&node_type) || path.len() >= T {
            return;
        }

        path.push(node_type);

        // add current path to output if it reached the desired length
        if terminals.contains(node_type) {
            out.push(path.clone());
        } else if let Some(parents) = backmap.get(node_type) {
            for parent in parents {
                helper::<T>(*parent, path, terminals, types, backmap, out);
            }
        }

        // remove self from the path before returning
        path.pop();
    }

    paths
}