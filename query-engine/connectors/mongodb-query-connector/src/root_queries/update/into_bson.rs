use super::expression::*;
use crate::IntoBson;
use connector_interface::FieldPath;
use mongodb::bson::{doc, Bson, Document};

impl IntoBson for Set {
    fn into_bson(self) -> crate::Result<Bson> {
        let doc = doc! {
            "$set": { self.field_path.path(true): (*self.expression).into_bson()? }
        };

        Ok(Bson::from(doc))
    }
}

impl IntoBson for IfThenElse {
    fn into_bson(self) -> crate::Result<Bson> {
        let doc = doc! {
            "$cond": {
                "if": (*self.cond).into_bson()?,
                "then": (*self.then).into_bson()?,
                "else": (*self.els).into_bson()?
            }
        };

        Ok(Bson::from(doc))
    }
}

impl IntoBson for Upsert {
    /// Renders an `Upsert` expression into `Bson`.
    ///
    /// Given the following upsert query:
    /// ```text
    /// {
    ///   opt_field: {
    ///     upsert: {
    ///       set: { a: 1 },
    ///       update: { a: 2 }
    ///     }
    ///   }
    /// }
    /// ```
    /// The `Upsert` struct will _roughly_ be filled as such:
    /// ```text
    /// Upsert {
    ///   field_path: vec!["opt_field"]
    ///   set: Set {
    ///     field_path: vec!["opt_field"],
    ///     expression: doc! { "$literal": { "a": 1 } }
    ///   },
    ///   updates: vec![
    ///     Set {
    ///       field_path: vec!["opt_field", "a"],
    ///       expression: doc! { "$literal": 1 }
    ///     }
    ///   ]
    /// }
    /// And _roughly_ transformed into `Bson` as such:
    /// Bson::Array(vec![
    ///   doc! { "addFields": { "__prisma_should_set__": true|false } },
    ///   doc! { "set": { "opt_field": { "$cond": { "if": { "$eq": ["$__prisma_should_set__", true] }, "then": { "$literal": { "a": 1 } }, "else": "$opt_some_field" } } } },
    ///   doc! { "set": { "opt_field.a": { "$cond": { "if": { "$eq": ["$__prisma_should_set__", false] }, "then": { "$literal": 1 }, "else": "$opt_some_field.a" } } } },
    ///   doc! { "unset": "__prisma_shhould_set" }
    /// ])
    /// ```
    fn into_bson(self) -> crate::Result<Bson> {
        let should_set_id = format!("__prisma_should_set__{}", &self.field_path.identifier());
        let should_set_ref_id = format!("${}", &should_set_id);

        let mut docs = vec![];

        // Adds a custom field to compute whether {field_path} should be set or updated
        docs.push(Bson::from(
            doc! { "$addFields": { &should_set_id: Upsert::render_should_set_condition(self.field_path()) }},
        ));

        // Maps the `Set` expression so that it's only executed if the field should be set. eg:
        // From: { $set: { {field_path}: {some_expression} } }
        // To:   { $set: { $cond: { if: {cond}, then: {some_expression}, else: "${field_path}"  } } }
        // where {cond} is the expression is the `cond` variable above
        docs.push(
            self.set
                .into_conditional_set(doc! { "$eq": [&should_set_ref_id, true] })
                .into_bson()?,
        );

        let updates = self
            .updates
            .into_iter()
            .map(|expr| match expr {
                UpdateExpression::Set(set) => {
                    // Maps the `Set` expression so that it's only executed if the field should be updated. eg:
                    // From: { $set: { {field_path}: {some_expression} } }
                    // To:   { $set: { $cond: { if: {cond}, then: {some_expression}, else: "${field_path}"  } } }
                    // where {cond} is the expression is the `cond` variable above
                    set.into_conditional_set(doc! { "$eq": [&should_set_ref_id, false] })
                        .into_bson()
                }
                expr => expr.into_bson(),
            })
            .collect::<crate::Result<Vec<_>>>()?;

        docs.extend(updates);

        // Removes the custom field previously added
        docs.push(Bson::from(doc! { "$unset": Bson::from(should_set_id) }));

        Ok(Bson::Array(docs))
    }
}

impl IntoBson for UpdateMany {
    fn into_bson(self) -> crate::Result<Bson> {
        dbg!(&self.field_path());

        let dollar_path = self.field_path().dollar_path(true);
        // The alias that will be used in the `$map` operation
        let elem_alias = self.elem_alias.clone();
        // A reference to that alias
        let ref_elem_alias = format!("$${}", &elem_alias);

        // Builds a `$mergeObjects` operation to perform updates on each element of the to-many embeds.
        // The `FieldPath` used for the `$mergeObjects` is the alias constructed above, since we'll merge against
        // each elements of the to-many embeds. eg:
        // { "$mergeObjects": ["$$some_field_item", { ... }] }
        let merge_objects = MergeObjects::new(
            FieldPath::new_from_alias(&format!("${}", &elem_alias)),
            self.updates,
        );

        let update_many = doc! {
            "$map": {
                "input": dollar_path,
                "as": elem_alias,
                "in": UpdateExpression::if_then_else(
                    doc! { "$eq": [true, true] }, // TODO: stub predicate until read filters are done
                    merge_objects.into_bson()?,
                    Bson::String(ref_elem_alias)
                ).into_bson()?
            }
        };

        Ok(Bson::from(update_many))
    }
}

impl IntoBson for MergeObjects {
    fn into_bson(self) -> crate::Result<Bson> {
        let doc: crate::Result<Document> =
            self.expressions
                .into_iter()
                .try_fold(Document::default(), |mut merge_doc, update| match update {
                    UpdateExpression::Set(set) => {
                        MergeObjects::merge_set(&mut merge_doc, set)?;

                        Ok(merge_doc)
                    }
                    UpdateExpression::Upsert(upsert) => {
                        MergeObjects::merge_upsert(&mut merge_doc, upsert, 0)?;

                        Ok(merge_doc)
                    }
                    _ => unreachable!(),
                });

        Ok(Bson::from(doc! { "$mergeObjects": [self.field_path.dollar_path(true), doc?] }))
    }
}

impl IntoBson for UpdateExpression {
    fn into_bson(self) -> crate::Result<Bson> {
        match self {
            UpdateExpression::Set(set) => set.into_bson(),
            UpdateExpression::IfThenElse(if_then_else) => if_then_else.into_bson(),
            UpdateExpression::Upsert(upsert) => upsert.into_bson(),
            UpdateExpression::UpdateMany(update_many) => update_many.into_bson(),
            UpdateExpression::Generic(bson) => Ok(bson),
        }
    }
}
