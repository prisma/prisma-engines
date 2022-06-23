use crate::join_utils::{self, AliasedJoin};
use crate::model_extensions::{AsColumn, AsTable, RelationFieldExt};
use connector_interface::NestedRead;
use prisma_models::{ModelRef, RelationFieldRef};
use quaint::ast::*;
use quaint::prelude::{Column, Expression};

#[derive(Debug)]
pub struct NestedReadJoins {
    pub joins: Vec<Join>,
    pub columns: Vec<Expression<'static>>,
}

pub fn build_joins(nested: &[NestedRead]) -> NestedReadJoins {
    let mut joins: Vec<Join> = vec![];
    let mut columns: Vec<Expression<'static>> = vec![];

    for (i, read) in nested.iter().enumerate() {
        let join = if read.parent_field.relation().is_one_to_many() {
            compute_one2m_join(&read.parent_field)
        } else if read.parent_field.relation().is_one_to_one() {
            compute_one2m_join(&read.parent_field)
        } else {
            todo!("m2m not supported yet")
        };

        for (i, selection) in read.selected_fields.selections().enumerate() {
            let col: Expression = Column::from((
                read.parent_field.related_model().as_table(),
                selection.as_scalar().unwrap().as_column(),
            ))
            .into();

            columns.push(col.alias(read.db_alias(i)));
        }

        let nested = build_joins(&read.nested);

        joins.push(join);

        joins.extend(nested.joins);
        columns.extend(nested.columns);
    }

    NestedReadJoins { joins, columns }
}

#[derive(Debug)]
pub struct Join {
    pub(crate) data: JoinData<'static>,
}

pub fn compute_one2m_join(rf: &RelationFieldRef) -> Join {
    let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
        (rf.scalar_fields(), rf.referenced_fields())
    } else {
        (
            rf.related_field().referenced_fields(),
            rf.related_field().scalar_fields(),
        )
    };

    let related_model = rf.related_model();
    let pairs = left_fields.into_iter().zip(right_fields.into_iter());

    // TODO: Add nested filter here
    let on_conditions: Vec<Expression> = pairs
        .map(|(a, b)| {
            let a_col = a.as_column();
            let b_col = b.as_column();

            a_col.equals(b_col).into()
        })
        .collect::<Vec<_>>();

    Join {
        data: related_model.as_table().on(ConditionTree::And(on_conditions)),
    }
}
