use super::{expression::*, operation::*};
use crate::{filter, IntoBson};

use itertools::Itertools;
use bson::{doc, Bson};

pub(crate) trait IntoUpdateExpression {
    fn into_update_expression(self) -> crate::Result<UpdateExpression>;
}

pub(crate) trait IntoUpdateExpressions {
    fn into_update_expressions(self) -> crate::Result<Vec<UpdateExpression>>;
}

impl IntoUpdateExpressions for UpdateOperation {
    fn into_update_expressions(self) -> crate::Result<Vec<UpdateExpression>> {
        match self {
            UpdateOperation::Generic(generic) => Ok(vec![generic.into_update_expression()?]),
            UpdateOperation::UpdateMany(update_many) => Ok(vec![update_many.into_update_expression()?]),
            UpdateOperation::Unset(unset) => Ok(vec![unset.into_update_expression()?]),
            UpdateOperation::Upsert(upsert) => upsert.into_update_expressions(),
        }
    }
}

impl IntoUpdateExpression for GenericOperation {
    fn into_update_expression(self) -> crate::Result<UpdateExpression> {
        Ok(UpdateExpression::set(self.field_path, self.expression))
    }
}

impl IntoUpdateExpressions for Upsert {
    fn into_update_expressions(self) -> crate::Result<Vec<UpdateExpression>> {
        let should_set_id = format!("__prisma_should_set__{}", &self.field_path.identifier());
        let should_set_ref_id = format!("${}", &should_set_id);

        let mut expressions: Vec<UpdateExpression> = vec![];

        // Adds a custom field to compute whether {field_path} should be set or updated
        expressions.push(
            doc! { "$addFields": { &should_set_id: Upsert::render_should_set_condition(self.field_path()) }}.into(),
        );

        // Maps the `Set` expression so that it's only executed if the field should be set. eg:
        // From: { $set: { {field_path}: {some_expression} } }
        // To:   { $set: { $cond: { if: {cond}, then: {some_expression}, else: "${field_path}"  } } }
        // where {cond} is the expression passed to `into_conditional_set`
        expressions.push(
            Set::from(self.set)
                .into_conditional_set(doc! { "$eq": [&should_set_ref_id, true] })
                .into(),
        );

        for op in self.updates {
            match op {
                // Since `Upsert` operations are transformed into a list of conditional sets,
                // there's no need to transform them twice so we just append them to the list of expressions.
                UpdateOperation::Upsert(upsert) => {
                    expressions.extend(upsert.into_update_expressions()?);
                }
                operation => {
                    let exprs = operation
                        .into_update_expressions()?
                        .into_iter()
                        .map(|expr| {
                            // Maps the `Set` expression so that it's only executed if the field should be updated. eg:
                            // From: { $set: { {field_path}: {some_expression} } }
                            // To:   { $set: { $cond: { if: {cond}, then: {some_expression}, else: "${field_path}"  } } }
                            // where {cond} is the expression passed to `into_conditional_set`
                            let set = expr
                                .try_into_set()
                                .expect("all upsert's update expressions should be `Set`s")
                                .into_conditional_set(doc! { "$eq": [&should_set_ref_id, false] });

                            UpdateExpression::from(set)
                        })
                        .collect_vec();

                    expressions.extend(exprs);
                }
            }
        }

        // Removes the custom field previously added
        expressions.push(doc! { "$unset": Bson::String(should_set_id) }.into());

        Ok(expressions)
    }
}

impl IntoUpdateExpression for UpdateMany {
    fn into_update_expression(self) -> crate::Result<UpdateExpression> {
        let field_path = self.field_path.clone();
        // The alias that will be used in the `$map` operation
        let elem_alias = self.elem_alias.clone();
        // A reference to that alias
        let ref_elem_alias = format!("$${}", &elem_alias);

        let (filter_doc, _) = filter::MongoFilterVisitor::new(format!("${}", &elem_alias), false)
            .visit(self.filter.clone())?
            .render();

        // Builds a `$mergeObjects` operation to perform updates on each element of the to-many embeds.
        // The `FieldPath` used for the `$mergeObjects` is the alias constructed above, since we'll merge against
        // each elements of the to-many embeds. eg:
        // { "$mergeObjects": ["$$some_field_item", { ... }] }
        let merge_objects = self.into_merge_objects_expr()?;

        let map_expr = doc! {
            "$map": {
                "input": field_path.dollar_path(true),
                "as": &elem_alias,
                "in": {
                    "$cond": {
                        "if": filter_doc,
                        "then": merge_objects.into_bson()?,
                        "else": Bson::String(ref_elem_alias)
                    }
                }
            }
        };

        Ok(UpdateExpression::set(field_path, map_expr))
    }
}

impl IntoUpdateExpression for Unset {
    fn into_update_expression(self) -> crate::Result<UpdateExpression> {
        Ok(UpdateExpression::set(self.field_path, Bson::from("$$REMOVE")))
    }
}
