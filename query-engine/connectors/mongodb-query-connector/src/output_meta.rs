use connector_interface::AggregationSelection;
use datamodel::FieldArity;
use prisma_models::{ModelProjection, PrismaValue, ScalarFieldRef, TypeIdentifier};
use std::collections::HashMap;

// let mut idents = selected_fields.type_identifiers_with_arities();

/// Maps field db field names to their meta information.
pub type OutputMetaMapping = HashMap<String, OutputMeta>;

pub struct OutputMeta {
    pub ident: TypeIdentifier,
    pub default: Option<PrismaValue>,
    pub list: bool,
}

impl OutputMeta {
    pub fn strip_list(&self) -> OutputMeta {
        OutputMeta {
            ident: self.ident.clone(),
            default: self.default.clone(),
            list: false,
        }
    }
}

pub fn from_selected_fields(selected_fields: &ModelProjection) -> OutputMetaMapping {
    let mut map = OutputMetaMapping::new();

    for field in selected_fields.scalar_fields() {
        map.insert(field.db_name().to_owned(), from_field(&field));
    }

    map
}

pub fn from_field(field: &ScalarFieldRef) -> OutputMeta {
    let (ident, field_arity) = field.type_identifier_with_arity();

    // Only add a possible default return if the field is required.
    let default = field.default_value.clone().and_then(|dv| match dv {
        datamodel::DefaultValue::Single(pv) if field.is_required => Some(pv),
        _ => None,
    });

    OutputMeta {
        ident,
        default,
        list: matches!(field_arity, FieldArity::List),
    }
}

/// Mapping valid for one specific selection.
/// Field name -> OutputMeta
pub fn from_aggregation_selection(selection: &AggregationSelection) -> OutputMetaMapping {
    let mut map = OutputMetaMapping::new();

    for (name, ident, field_arity) in selection.identifiers() {
        map.insert(
            name,
            OutputMeta {
                ident,
                default: None,
                list: matches!(field_arity, FieldArity::List),
            },
        );
    }

    map
}
