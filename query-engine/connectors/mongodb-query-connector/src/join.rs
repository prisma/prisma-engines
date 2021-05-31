use mongodb::bson::{doc, Document};
use prisma_models::RelationFieldRef;

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
}

impl JoinStage {
    pub(crate) fn new(source: RelationFieldRef) -> Self {
        Self {
            source,
            alias: None,
            nested: vec![],
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
    /// `{ _id: 1, field: "a", alisHere: [{...}, {...}, ...] }` with alias `"aliasHere"`.
    ///
    /// Returns: `(Join document, Unwind document)`
    pub(crate) fn build(self) -> (Document, Option<Document>) {
        let nested_stages: Vec<Document> = self
            .nested
            .into_iter()
            .flat_map(|nested_stage| {
                let (join, unwind) = nested_stage.build();

                match unwind {
                    Some(unwind) => vec![join, unwind],
                    None => vec![join],
                }
            })
            .collect();

        let from_field = self.source;
        let relation = from_field.relation();
        let as_name = if let Some(alias) = self.alias {
            alias
        } else {
            relation.name.clone()
        };

        let right_model = from_field.related_model();
        let right_coll_name = right_model.db_name();

        let mut left_scalars = from_field.left_scalars();
        let mut right_scalars = from_field.right_scalars();

        // +1 for the required match stage, the rest is from the joins.
        let mut pipeline = Vec::with_capacity(1 + nested_stages.len());

        // todo: multi-field joins
        // Field on the right hand collection of the join.
        let right_field = right_scalars.pop().unwrap();
        let right_name = right_field.db_name().to_string();

        // Field on the left hand collection of the join.
        let left_field = left_scalars.pop().unwrap();
        let left_name = left_field.db_name();

        let right_ref = format!("${}", right_name);
        let op = if relation.is_many_to_many() {
            // For m-n join stages: Add an `$addFields` stage that adds an empty array if not present (required to make joins work).
            pipeline.push(doc! {
                "$addFields": {
                    right_name: {
                        "$cond": {
                            "if": {
                                "$ne": [ { "$type": right_ref.clone() }, "array" ]
                            },
                            "then": [],
                            "else": right_ref.clone(),
                        }
                    }
                }
            });

            if right_field.is_list {
                doc! { "$in": ["$$left", right_ref] }
            } else {
                doc! { "$in": [right_ref, "$$left"] }
            }
        } else {
            doc! { "$eq": [right_ref, "$$left"] }
        };

        pipeline.push(doc! { "$match": { "$expr": op }});
        pipeline.extend(nested_stages);

        // Todo: Temporarily disabled.
        // If the field is a to-one, add and unwind stage.
        // let unwind_stage = if !from_field.is_list {
        //     Some(doc! {
        //         "$unwind": { "path": format!("${}", as_name), "preserveNullAndEmptyArrays": true }
        //     })
        // } else {
        //     None
        // };
        let unwind_stage = None;

        let join_stage = doc! {
            "$lookup": {
                "from": right_coll_name,
                "let": { "left": format!("${}", left_name) },
                "pipeline": pipeline,
                "as": as_name,
            }
        };

        (join_stage, unwind_stage)
    }
}
