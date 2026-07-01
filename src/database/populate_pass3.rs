use crate::{Database, DatabaseError, Id, parser::TypeBroker};

impl Database {
    pub fn populate_pass3_link(&mut self, mut broker: TypeBroker) -> Result<(), DatabaseError> {
        let ids = self.assets.keys().cloned().collect::<Vec<_>>();
        broker.fulfill(ids.iter(), self);
        Ok(())
    }
}
