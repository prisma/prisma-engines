use super::default_value;
use crate::validate::validation_pipeline::context::Context;
use diagnostics::DatamodelError;
use parser_database::{
    ast::{self, WithSpan},
    walkers::{CompositeTypeFieldWalker, CompositeTypeWalker},
    ScalarFieldType,
};
use std::{fmt, rc::Rc};

/// Detect compound type chains that form a cycle, that is not broken with either an optional or an
/// array type.
pub(super) fn detect_composite_cycles(ctx: &mut Context<'_>) {
    let mut visited: Vec<ast::CompositeTypeId> = Vec::new();
    let mut errors: Vec<(ast::CompositeTypeId, DatamodelError)> = Vec::new();

    let mut fields_to_traverse: Vec<(CompositeTypeFieldWalker<'_>, Option<Rc<CompositeTypePath<'_>>>)> = ctx
        .db
        .walk_composite_types()
        .flat_map(|ct| ct.fields())
        .filter(|f| f.arity().is_required())
        .map(|f| (f, None))
        .collect();

    while let Some((field, path)) = fields_to_traverse.pop() {
        let path = match path {
            Some(path) => path,
            None => {
                visited.clear();
                Rc::new(CompositeTypePath::root(field.composite_type()))
            }
        };

        match field.r#type() {
            ScalarFieldType::CompositeType(ctid) if field.composite_type().composite_type_id() == ctid => {
                let msg = "The type is the same as the parent and causes an endless cycle. Please change the field to be either optional or a list.";
                errors.push((
                    ctid,
                    DatamodelError::new_composite_type_field_validation_error(
                        msg,
                        field.composite_type().name(),
                        field.name(),
                        field.ast_field().span(),
                    ),
                ));
            }
            ScalarFieldType::CompositeType(ctid) if visited.first() == Some(&ctid) => {
                let msg = format!(
                    "The types cause an endless cycle in the path {path}. Please change one of the fields to be either optional or a list to break the cycle."
                );

                errors.push((
                    ctid,
                    DatamodelError::new_composite_type_field_validation_error(
                        &msg,
                        field.composite_type().name(),
                        field.name(),
                        field.ast_field().span(),
                    ),
                ));
            }
            ScalarFieldType::CompositeType(ctid) => {
                visited.push(ctid);

                for field in ctx.db.walk(ctid).fields().filter(|f| f.arity().is_required()) {
                    let path = Rc::new(path.link(field.composite_type()));
                    fields_to_traverse.push((field, Some(path)));
                }
            }
            _ => (),
        }
    }

    errors.sort_by_key(|(id, _err)| *id);
    for (_, error) in errors {
        ctx.push_error(error);
    }
}

/// Does the connector support composite types.
pub(crate) fn composite_types_support(composite_type: CompositeTypeWalker<'_>, ctx: &mut Context<'_>) {
    if ctx.connector.supports_composite_types() {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        &format!("Composite types are not supported on {}.", ctx.connector.name()),
        composite_type.ast_composite_type().span,
    ));
}

/// A composite type must have at least one field.
pub(crate) fn more_than_one_field(composite_type: CompositeTypeWalker<'_>, ctx: &mut Context<'_>) {
    let num_of_fields = composite_type.fields().count();

    if num_of_fields > 0 {
        return;
    }

    ctx.push_error(DatamodelError::new_validation_error(
        "A type must have at least one field defined.",
        composite_type.ast_composite_type().span,
    ));
}

/// Validates the @default attribute of a composite scalar field
pub(super) fn validate_default_value(field: CompositeTypeFieldWalker<'_>, ctx: &mut Context<'_>) {
    let default_value = field.default_value();
    let default_attribute = field.default_attribute();

    if field.default_mapped_name().is_some() {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "A `map` argument for the default value of a field on a composite type is not allowed. Consider removing it.",
            "@default",
            default_attribute.unwrap().span,
        ));
    }

    let scalar_type = field.r#type().as_builtin_scalar();

    default_value::validate_default_value(default_value, scalar_type, ctx);
}

struct CompositeTypePath<'db> {
    previous: Option<Rc<CompositeTypePath<'db>>>,
    current: CompositeTypeWalker<'db>,
}

impl<'db> CompositeTypePath<'db> {
    fn root(current: CompositeTypeWalker<'db>) -> Self {
        Self {
            previous: None,
            current,
        }
    }

    fn link(self: &Rc<Self>, current: CompositeTypeWalker<'db>) -> Self {
        Self {
            previous: Some(self.clone()),
            current,
        }
    }
}

impl<'db> fmt::Display for CompositeTypePath<'db> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut traversed = vec![self.current];
        let mut this = self;

        while let Some(next) = this.previous.as_ref() {
            traversed.push(next.current);
            this = next;
        }

        let path = traversed
            .into_iter()
            .map(|w| w.name())
            .map(|n| format!("`{n}`"))
            .collect::<Vec<_>>()
            .join(" → ");

        f.write_str(&path)
    }
}
