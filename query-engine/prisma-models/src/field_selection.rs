use crate::{CompositeFieldRef, ModelRef, ScalarFieldRef};

/// A selection of fields from a model.
pub struct FieldSelection {
    pub model: ModelRef,
    pub selection: Vec<SelectedField>,
}

/// A selected field. Can be contained on a model or composite type.
pub enum SelectedField {
    Scalar(ScalarFieldRef),
    Composite(CompositeSelection),
}

pub struct CompositeSelection {
    pub field: CompositeFieldRef,
    pub selections: Vec<SelectedField>,
}

impl FieldSelection {}

impl From<ScalarFieldRef> for SelectedField {
    fn from(f: ScalarFieldRef) -> Self {
        Self::Scalar(f)
    }
}

impl From<(ModelRef, ScalarFieldRef)> for FieldSelection {
    fn from((model, field): (ModelRef, ScalarFieldRef)) -> Self {
        Self {
            model,
            selection: vec![field.into()],
        }
    }
}
