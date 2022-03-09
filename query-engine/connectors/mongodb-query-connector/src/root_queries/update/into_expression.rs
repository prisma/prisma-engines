use crate::IntoBson;

use super::{expression::*, operation::*};

use mongodb::bson::{doc, Bson};

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
        // where {cond} is the expression is the `cond` variable above
        expressions.push(
            Set::from(self.set)
                .into_conditional_set(doc! { "$eq": [&should_set_ref_id, true] })
                .into(),
        );

        let mut updates = vec![];

        for op in self.updates {
            match op {
                UpdateOperation::Generic(generic) => {
                    // Maps the `Set` expression so that it's only executed if the field should be updated. eg:
                    // From: { $set: { {field_path}: {some_expression} } }
                    // To:   { $set: { $cond: { if: {cond}, then: {some_expression}, else: "${field_path}"  } } }
                    // where {cond} is the expression is the `cond` variable above
                    let set = Set::from(generic).into_conditional_set(doc! { "$eq": [&should_set_ref_id, false] });

                    updates.push(set.into());
                }
                operation => {
                    updates.extend(operation.into_update_expressions()?);
                }
            }
        }

        expressions.extend(updates);

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

        // Builds a `$mergeObjects` operation to perform updates on each element of the to-many embeds.
        // The `FieldPath` used for the `$mergeObjects` is the alias constructed above, since we'll merge against
        // each elements of the to-many embeds. eg:
        // { "$mergeObjects": ["$$some_field_item", { ... }] }
        let merge_doc = self.build_merge_doc()?;

        let map_expr = doc! {
            "$map": {
                "input": field_path.dollar_path(true),
                "as": &elem_alias,
                "in": {
                    "$cond": {
                        "if": doc! { "$eq": [true, true] }, // TODO: stub predicate until read filters are done,
                        "then": doc! { "$mergeObjects": [&ref_elem_alias, merge_doc.into_bson()?] },
                        "else": Bson::String(ref_elem_alias)
                    }
                }
            }
        };

        Ok(UpdateExpression::set(field_path, map_expr))
    }
}
