use crate::StatefulList;

use super::client::GlueTable;

pub struct ViewModel {
    pub list: StatefulList<GlueTable>,
}

impl ViewModel {
    pub fn new(tables: Vec<GlueTable>) -> Self {
        Self {
            list: StatefulList::new(tables),
        }
    }
}
