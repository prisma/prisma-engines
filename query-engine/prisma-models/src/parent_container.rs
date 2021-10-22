use crate::{CompositeTypeWeakRef, ModelWeakRef};

pub enum ParentContainer {
    Model(ModelWeakRef),
    CompositeType(CompositeTypeWeakRef),
}
