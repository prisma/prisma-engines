use crate::queryable::NodeJSQueryable;

pub struct NodeJSPool {
    pub nodejs_queryable: NodeJSQueryable,
}

impl NodeJSPool {
    pub fn new(nodejs_queryable: NodeJSQueryable) -> Self {
        Self { nodejs_queryable }
    }
}
