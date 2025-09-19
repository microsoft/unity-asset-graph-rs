use crate::{
    Database, 
    DatabaseError,
    Id,
    parser::TypeBroker,
};

impl Database {
    pub fn populate_pass3_link(&mut self, mut broker: TypeBroker) -> Result<(), DatabaseError> {
        broker.fulfill_known(self);

        let ids: Vec<Id> = self.assets.keys().cloned().collect();
        for id in &ids {
            broker.fulfill(id, self);
        }
        Ok(())
    }
}