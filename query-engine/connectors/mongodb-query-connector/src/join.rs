use crate::filter::MongoFilter;
use mongodb::bson::{doc, Document};
use query_structure::{walkers, RelationFieldRef, ScalarFieldRef};

/// A join stage describes a tree of joins and nested joins to be performed on a collection.
/// Every document of the `source` side will be joined with the collection documents
/// as described by the relation field. All of the newly joined documents
/// can be joined again with relations originating from their collection.
/// Example:
/// ```text
/// A -> B -> C
///        -> D
/// ```
/// Translates to: `JoinStage(A, nested: Vec<JoinStage(B), JoinStage(C)>)`.
#[derive(Debug, Clone)]
pub(crate) struct JoinStage {
    /// The starting point of the traversal (left model of the join).
    pub(crate) source: RelationFieldRef,

    /// By default, the name of the relation is used as join field name.
    /// Can be overwritten with an alias here.
    pub(crate) alias: Option<String>,

    /// Nested joins
    pub(crate) nested: Vec<JoinStage>,

    /// Filter on the join itself, used for aggregations on relations.
    pub(crate) filter: Option<MongoFilter>,
}

impl JoinStage {
    pub(crate) fn new(source: RelationFieldRef) -> Self {
        Self {
            source,
            alias: None,
            nested: vec![],
            filter: None,
        }
    }

    pub(crate) fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }

    pub(crate) fn push_nested(&mut self, stage: JoinStage) {
        self.nested.push(stage);
    }

    pub(crate) fn extend_nested(&mut self, stages: Vec<JoinStage>) {
        self.nested.extend(stages);
    }

    /// Returns a join stage for the join between the source collection of `from_field` (the model it's defined on)
    /// and the target collection (the model that is related over the relation), as well as an optional unwind stage.
    /// The joined documents will reside on the source document side as a field **named after the relation name** if
    /// there's no `alias` defined. Else the alias is the name.
    /// Example: If you have a document `{ _id: 1, field: "a" }` and join relation "aToB", the resulting document
    /// will have the shape: `{ _id: 1, field: "a", aToB: [{...}, {...}, ...] }` without alias and
    /// `{ _id: 1, field: "a", aliasHere: [{...}, {...}, ...] }` with alias `"aliasHere"`.
    ///
    /// Returns: `(Join document, Unwind document)`
    pub(crate) fn build(self) -> (Document, Option<Document>) {
        let (filter_doc, filter_joins) = self
            .filter
            .map(MongoFilter::render)
            .map(|(doc, joins)| (Some(doc), joins))
            .unwrap_or_else(|| (None, vec![]));

        let nested_stages: Vec<Document> = self
            .nested
            .into_iter()
            .chain(filter_joins)
            .map(|nested_stage| {
                let (join, _) = nested_stage.build();

                join
            })
            .collect();

        let from_field = self.source;
        let relation = from_field.relation();
        let as_name = if let Some(alias) = self.alias {
            alias
        } else {
            relation.name()
        };

        let right_model = from_field.related_model();
        let right_coll_name = right_model.db_name();

        // +1 for the required match stage, the rest is from the joins.
        let mut pipeline = Vec::with_capacity(1 + nested_stages.len());

        // First we start with the right side of the equation
        let right_scalars: Vec<ScalarFieldRef> = match from_field.walker().relation().refine() {
            walkers::RefinedRelationWalker::Inline(_) | walkers::RefinedRelationWalker::ImplicitManyToMany(_) => {
                from_field.related_field().left_scalars()
            }
            walkers::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_) => from_field.related_field().scalar_fields(),
        };

        // What $expr operators we will need to express this lookup? (depends on right fields)
        let ops: Vec<Document> = right_scalars
            .iter()
            .enumerate()
            .map(|(idx, right_field)| {
                let right_ref = format!("${}", right_field.db_name());
                let left_var = format!("$$left_{idx}");

                match relation.is_many_to_many() {
                    true if right_field.is_list() => doc! { "$in": [left_var, right_ref] },
                    true => doc! { "$in": [right_ref, left_var] },
                    _ => doc! { "$eq": [right_ref, left_var] },
                }
            })
            .collect();

        // For m-n join stages: Add an `$addFields` stage that adds an empty array if not present (required to make joins work).
        if relation.is_many_to_many() {
            // addFields is the list of fields and conditions
            let mut add_fields = Document::new();

            // Go through every right field to place in the $addFields operator
            for right_field in right_scalars.iter() {
                let right_name = right_field.db_name();
                let right_ref = format!("${right_name}");

                add_fields.insert(
                    right_name,
                    doc! {
                       "$cond": {
                            "if": {
                                "$ne": [ { "$type": right_ref.clone() }, "array"]
                            },
                            "then": [],
                            "else": right_ref.clone()
                        }
                    },
                );
            }

            // Push addFields to pipeline
            pipeline.push(doc! {
                "$addFields": add_fields
            });
        }

        // We can now express the match from the operators
        pipeline.push(doc! { "$match": { "$expr": { "$and": ops } }});

        pipeline.extend(nested_stages);

        // Add inner join filters if there are any (used for relational aggregations)
        if let Some(doc) = filter_doc {
            pipeline.push(doc! { "$match": { "$expr": doc } });
        }

        // If the field is a to-one, add an unwind stage.
        let unwind_stage = if !from_field.is_list() {
            Some(doc! {
                "$unwind": { "path": format!("${as_name}"), "preserveNullAndEmptyArrays": true }
            })
        } else {
            None
        };

        // Time to deal with the left side of the equation
        let left_scalars = from_field.left_scalars();

        let mut let_vars = Document::new();

        // With the left side, we need to introduce the variable `left_x` pointing to the correct field
        for (idx, left_field) in left_scalars.iter().enumerate() {
            let left_var = format!("left_{idx}");

            let_vars.insert(left_var, format!("${}", left_field.db_name()));
        }

        // We can now generate the full $lookup query with all its parts
        let join_stage = doc! {
            "$lookup": {
                "from": right_coll_name,
                "let": let_vars,
                "pipeline": pipeline,
                "as": as_name,
            }
        };

        (join_stage, unwind_stage)
    }
}
