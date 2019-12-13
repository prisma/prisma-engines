use crate::{AsColumn, Relation, RelationExt, SelectedFields};
use quaint::ast::Column;

pub trait SelectedFieldsExt {
    fn columns(&self) -> Vec<Column<'static>>;
}

impl SelectedFieldsExt for SelectedFields {
    fn columns(&self) -> Vec<Column<'static>> {
        let mut result: Vec<Column<'static>> = self.scalar_non_list().iter().map(|f| f.as_column()).collect();

        for rf in self.relation_inlined().iter() {
            result.push(rf.as_column());
        }

        if let Some(ref from_field) = self.from_field {
            let relation = from_field.relation();

            if from_field.relation_is_inlined_in_child() {
                result.extend(
                    relation
                        .columns_for_relation_side(from_field.relation_side.opposite())
                        .into_iter()
                        .map(|col| col.alias(SelectedFields::RELATED_MODEL_ALIAS))
                        .collect::<Vec<Column<'static>>>(),
                );

                result.extend(
                    relation
                        .columns_for_relation_side(from_field.relation_side)
                        .into_iter()
                        .map(|col| col.alias(SelectedFields::PARENT_MODEL_ALIAS))
                        .collect::<Vec<Column<'static>>>(),
                );
            } else {
                result.extend(
                    relation
                        .columns_for_relation_side(from_field.relation_side.opposite())
                        .into_iter()
                        .map(|col| {
                            col.alias(SelectedFields::RELATED_MODEL_ALIAS)
                                .table(Relation::TABLE_ALIAS)
                        })
                        .collect::<Vec<Column<'static>>>(),
                );

                result.extend(
                    relation
                        .columns_for_relation_side(from_field.relation_side)
                        .into_iter()
                        .map(|col| {
                            col.alias(SelectedFields::PARENT_MODEL_ALIAS)
                                .table(Relation::TABLE_ALIAS)
                        })
                        .collect::<Vec<Column<'static>>>(),
                );
            }
        };

        result
    }
}
