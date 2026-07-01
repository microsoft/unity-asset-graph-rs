use super::qualified_name::{QualifiedName, QualifiedNameOwned, QualifiedNamePart, QualifiedNameRef};
use crate::{Database, Id, Relation, parser::csharp::qualified_name::QualifiedNameSearchTree};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashSet, fmt::Display};

/// A reference to a type within the file being parsed. May be locally declared, fully qualified, or ambiguous and need brokering.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeRequest {
    /// The asset that uses the type
    pub requester: Id,
    /// The un- or partially-qualified name of the type being requested from the broker
    pub partial_name: QualifiedNameOwned,
    /// The namespaces in scope during the reference
    pub scoped_namespaces: Vec<QualifiedNameOwned>,
}

impl TypeRequest {
    /// Determines if the given type ID satisfies this type request.
    pub fn satisfied_by(&self, type_id: &Id) -> bool {
        let eeb = Id::CsType(QualifiedNameOwned::from(
            "Microsoft.Teams.Immersive.EventExperience.EventExperienceBoot",
        ));
        let roster = QualifiedNameOwned::from("IRosterProvider");
        if let Id::CsType(fqn) = type_id {
            let fqn = QualifiedNameRef::from(fqn);

            if &self.requester == &eeb && &self.partial_name == &roster {
                println!("Checking for {}: {fqn}", self.partial_name);
            }
            for i in 0..fqn.len() {
                let (ns, name) = fqn.split(i);
                if (ns.len() == 0 || self.scoped_namespaces.iter().any(|sns| *sns == ns)) && self.partial_name == name {
                    return true;
                }
            }
            false
        } else {
            false
        }
    }
}

impl Into<Id> for TypeRequest {
    fn into(self) -> Id {
        Id::CsType(self.partial_name)
    }
}

impl Display for TypeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} wants {}", self.requester, self.partial_name)?;
        writeln!(f, "  Namespaces in scope ({}):", self.scoped_namespaces.len())?;
        for ns in &self.scoped_namespaces {
            writeln!(f, "  - {ns}")?;
        }
        Ok(())
    }
}

/// A broker for managing type references during parsing. Tracks which types are declared and which are referenced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeBroker {
    requests: HashSet<TypeRequest>,
}

impl TypeBroker {
    pub fn new() -> Self {
        Self {
            requests: HashSet::new(),
        }
    }

    pub fn request(
        &mut self,
        requester: &Id,
        type_name: QualifiedNameOwned,
        scoped_namespaces: impl IntoIterator<Item = QualifiedNameOwned>,
    ) {
        self.requests.insert(TypeRequest {
            requester: requester.clone(),
            partial_name: type_name,
            scoped_namespaces: scoped_namespaces.into_iter().collect(),
        });
    }

    pub fn push(&mut self, request: TypeRequest) {
        self.requests.insert(request);
    }

    pub fn requests(&self) -> &HashSet<TypeRequest> {
        &self.requests
    }

    pub fn fulfill<'a>(&mut self, ids: impl ExactSizeIterator<Item = &'a Id>, database: &mut Database) {
        let tree = ids
            .filter_map(|id| {
                if let Id::CsType(name) = id {
                    Some(name)
                } else {
                    None
                }
            })
            .collect::<QualifiedNameSearchTree>();

        let mut matched_types = 0u32;
        print!("Matched types: {}", matched_types);
        self.requests
            .extract_if(|req| {
                for ns in &req.scoped_namespaces {
                    if let Some(tree) = tree.get(ns)
                        && tree.contains(&req.partial_name)
                    {
                        if let Some(asset) = database.asset_mut(&req.requester) {
                            asset
                                .relations
                                .insert(Relation::Uses(Id::CsType(QualifiedNameOwned::concat(
                                    ns,
                                    &req.partial_name,
                                ))));
                        }
                        matched_types += 1;
                        print!("\rMatched types: {}", matched_types);
                        return true;
                    }
                }
                false
            })
            .collect::<Vec<_>>();
        println!();
    }
}
