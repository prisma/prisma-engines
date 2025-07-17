use indexmap::IndexMap;
use query_structure::{
    AggregationSelection, DefaultKind, FieldSelection, PrismaValue, ScalarFieldRef, SelectedField, TypeIdentifier,
    ast::FieldArity,
};

/// Maps field db field names to their meta information.
pub type OutputMetaMapping = IndexMap<String, OutputMeta>;

/// `OutputMeta` contains information that is required to process the output of
/// Mongo queries. With this information, we can correctly parse information and
/// coerce values as necessary / fill missing data.
#[derive(Debug, Clone)]
pub enum OutputMeta {
    Scalar(ScalarOutputMeta),
    Composite(CompositeOutputMeta),
}

#[derive(Debug, Clone)]
pub struct ScalarOutputMeta {
    pub ident: TypeIdentifier,
    pub default: Option<PrismaValue>,
    pub list: bool,
}

impl ScalarOutputMeta {
    pub fn strip_list(&self) -> Self {
        Self {
            ident: self.ident,
            default: self.default.clone(),
            list: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompositeOutputMeta {
    pub list: bool,
    pub inner: OutputMetaMapping,
}

impl CompositeOutputMeta {
    pub fn strip_list(&self) -> Self {
        Self {
            list: false,
            inner: self.inner.clone(),
        }
    }
}

pub fn from_selected_fields(selected_fields: &FieldSelection) -> OutputMetaMapping {
    let selections: Vec<_> = selected_fields.selections().collect();
    from_selections(&selections)
}

pub fn from_selections(selected_fields: &[&SelectedField]) -> OutputMetaMapping {
    let mut map = OutputMetaMapping::new();

    for selection in selected_fields {
        match selection {
            SelectedField::Scalar(sf) => {
                map.insert(sf.db_name().to_owned(), from_scalar_field(sf));
            }

            SelectedField::Composite(cs) => {
                let selections: Vec<&SelectedField> = cs.selections.iter().collect();
                let inner = from_selections(&selections);

                map.insert(
                    cs.field.db_name().to_owned(),
                    OutputMeta::Composite(CompositeOutputMeta {
                        list: cs.field.is_list(),
                        inner,
                    }),
                );
            }

            SelectedField::Relation(_) => unreachable!(),

            SelectedField::Virtual(vs) => {
                let (ident, arity) = vs.type_identifier_with_arity();

                map.insert(
                    vs.db_alias(),
                    OutputMeta::Scalar(ScalarOutputMeta {
                        ident,
                        default: None,
                        list: matches!(arity, FieldArity::List),
                    }),
                );
            }
        }
    }

    map
}

pub fn from_scalar_field(field: &ScalarFieldRef) -> OutputMeta {
    let (ident, field_arity) = field.type_identifier_with_arity();

    // Only add a possible default return if the field is required.
    let default = field.default_value().and_then(|dv| match dv {
        DefaultKind::Single(pv) if field.is_required() => Some(pv),
        _ => None,
    });

    OutputMeta::Scalar(ScalarOutputMeta {
        ident,
        default,
        list: matches!(field_arity, FieldArity::List),
    })
}

/// Mapping valid for one specific selection.
/// Field name -> OutputMeta
pub fn from_aggregation_selection(selection: &AggregationSelection) -> OutputMetaMapping {
    let mut map = OutputMetaMapping::new();

    for ident in selection.identifiers() {
        map.insert(
            ident.db_name.into(),
            OutputMeta::Scalar(ScalarOutputMeta {
                ident: ident.typ.id,
                default: None,
                list: matches!(ident.arity, FieldArity::List),
            }),
        );
    }

    map
}

impl From<ScalarOutputMeta> for OutputMeta {
    fn from(s: ScalarOutputMeta) -> Self {
        Self::Scalar(s)
    }
}

impl From<CompositeOutputMeta> for OutputMeta {
    fn from(c: CompositeOutputMeta) -> Self {
        Self::Composite(c)
    }
}
