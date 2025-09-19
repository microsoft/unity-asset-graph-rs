use std::collections::HashSet;
use crate::{Database, Id, Relation};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TypeRequest {
    requester: Id,
    type_name: String,
    scoped_namespaces: Vec<String>,
    known: bool,
}

impl TypeRequest {
    pub fn new(requester: &Id, type_name: &str, scoped_namespaces: &Vec<String>, known: bool) -> Self {
        Self {
            requester: requester.clone(),
            type_name: type_name.into(),
            scoped_namespaces: scoped_namespaces.clone(),
            known,
        }
    }

    pub fn satisfied_by(&self, type_id: &Id) -> bool {
        if let Id::CsType { name, namespace } = type_id {
            if &self.type_name != name {
                return false;
            }
            if let Some(ns) = namespace {
                if self.scoped_namespaces.iter().any(|s| s == ns) {
                    return true;
                }
            } else if self.scoped_namespaces.is_empty() {
                return true;
            }
        }
        false
    }
}

impl std::convert::Into<Id> for TypeRequest {
    fn into(mut self) -> Id {
        if self.known {
            if let Some(ns) = self.scoped_namespaces.pop() {
                Id::CsType { name: self.type_name, namespace: Some(ns) }
            }
            else {
                Id::CsType { name: self.type_name, namespace: None }
            }
        }
        else {
            panic!()
        }
    }
}

pub struct TypeBroker {
    requests: HashSet<TypeRequest>,
}

impl TypeBroker {
    pub fn new() -> Self {
        Self {
            requests: HashSet::new(),
        }
    }

    pub fn request(&mut self, requester: &Id, type_name: &str, scoped_namespaces: &Vec<String>) {
        self.requests.insert(TypeRequest::new(requester, type_name, scoped_namespaces, false));
    }

    pub fn request_known(&mut self, requester: &Id, known: &Id) {
        if let Id::CsType { name, namespace } = known {
            self.requests.insert(TypeRequest::new(
                requester, 
                name, 
                &namespace.as_ref().map(|ns| vec![ns.clone()]).unwrap_or(vec![]), 
                true,
            ));
        }
        else {
            panic!("Id type not supported");
        }
    }

    pub fn fulfill(&mut self, id: &Id, database: &mut Database) {
        for req in self.requests.extract_if(|req| req.satisfied_by(id)) {
            if let Some(asset) = database.asset_mut(&req.requester) {
                asset.relations.insert(Relation::Uses(id.clone()));
            }
        }
    }

    pub fn fulfill_known(&mut self, database: &mut Database) {
        for req in self.requests.extract_if(|req| req.known) {
            if let Some(asset) = database.asset_mut(&req.requester) {
                asset.relations.insert(Relation::Uses(req.into()));
            }
        }
    }

    pub fn requests(&self) -> &HashSet<TypeRequest> {
        &self.requests
    }
}