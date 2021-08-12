use prisma_models::*;
use quaint::prelude::*;

#[derive(Debug, Clone)]
pub struct AliasedJoin {
    // Actual join data to be passed to quaint
    pub(crate) data: JoinData<'static>,
    // Alias used for the join. eg: LEFT JOIN ... AS <alias>
    pub(crate) alias: String,
}

#[derive(Debug, Clone)]
pub enum AggregationType {
    Count { _all: bool },
}

pub fn compute_aggr_join(
    rf: &RelationFieldRef,
    aggregation: AggregationType,
    aggregator_alias: &str,
    join_alias: &str,
    previous_join: Option<&AliasedJoin>,
) -> AliasedJoin {
    let join_alias = format!("{}_{}", join_alias, &rf.related_model().name);

    if rf.relation().is_many_to_many() {
        compute_aggr_join_m2m(rf, aggregation, aggregator_alias, join_alias.as_str(), previous_join)
    } else {
        compute_aggr_join_one2m(rf, aggregation, aggregator_alias, join_alias.as_str(), previous_join)
    }
}

fn compute_aggr_join_one2m(
    rf: &RelationFieldRef,
    sort_aggregation: AggregationType,
    aggregator_alias: &str,
    join_alias: &str,
    previous_join: Option<&AliasedJoin>,
) -> AliasedJoin {
    let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
        (rf.scalar_fields(), rf.referenced_fields())
    } else {
        (
            rf.related_field().referenced_fields(),
            rf.related_field().scalar_fields(),
        )
    };
    let select_columns = right_fields.iter().map(|f| f.as_column());

    // + SELECT A.fk FROM A
    let query = Select::from_table(rf.related_model().as_table()).columns(select_columns);
    let aggr_expr = match sort_aggregation {
        AggregationType::Count { _all } => count(asterisk()),
    };

    // SELECT A.fk,
    // + COUNT(*) AS <AGGREGATOR_ALIAS>
    // FROM A
    let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

    // SELECT A.<fk>, COUNT(*) AS <AGGREGATOR_ALIAS> FROM A
    // + GROUP BY A.<fk>
    let query = right_fields.iter().fold(query, |acc, f| acc.group_by(f.as_column()));

    let pairs = left_fields.into_iter().zip(right_fields.into_iter());
    let on_conditions = pairs
        .map(|(a, b)| {
            let col_a = match previous_join {
                Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                None => a.as_column(),
            };
            let col_b = Column::from((join_alias.to_owned(), b.db_name().to_owned()));

            col_a.equals(col_b)
        })
        .collect::<Vec<_>>();

    // + LEFT JOIN (
    //     SELECT A.<fk>, COUNT(*) AS <AGGREGATOR_ALIAS> FROM A
    //     GROUP BY A.<fk>
    // + ) AS <ORDER_JOIN_PREFIX> ON (<A | previous_join_alias>.<fk> = <ORDER_JOIN_PREFIX>.<fk>)
    let join = Table::from(query)
        .alias(join_alias.to_owned())
        .on(ConditionTree::single(on_conditions));

    AliasedJoin {
        data: join,
        alias: join_alias.to_owned(),
    }
}

fn compute_aggr_join_m2m(
    rf: &RelationFieldRef,
    aggregation: AggregationType,
    aggregator_alias: &str,
    join_alias: &str,
    previous_join: Option<&AliasedJoin>,
) -> AliasedJoin {
    let relation_table = rf.as_table();

    let model_a = rf.model();
    let model_a_alias = format!("{}_A", model_a.db_name());
    let a_ids = rf.model().primary_identifier();
    let a_columns: Vec<_> = a_ids.as_columns().map(|c| c.table(model_a_alias.clone())).collect();

    let model_b = rf.related_model();
    let model_b_alias = format!("{}_B", model_b.db_name());
    let b_ids = rf.related_model().primary_identifier();
    let b_columns: Vec<_> = b_ids.as_columns().map(|c| c.table(model_b_alias.clone())).collect();

    // SQL statements below refers to three tables:
    // A (left table): aliased as A_A
    // B (right table): aliased as B_B
    // _AtoB (join table): not aliased
    // A & B are aliased to support m2m self relations, in which case we inner join twice on the same table
    // If A & B are the "User" table, they'd be aliased to: User_A & User_B

    // + SELECT A_A.id FROM _AtoB
    let query = Select::from_table(relation_table).columns(a_columns.clone());

    let aggr_expr = match aggregation {
        AggregationType::Count { _all } => count(asterisk()),
    };
    // SELECT A_A.id,
    // + COUNT(*) AS <AGGREGATOR_ALIAS>
    // FROM _AtoB
    let query = query.value(aggr_expr.alias(aggregator_alias.to_owned()));

    let conditions_a: Vec<_> = a_columns
        .clone()
        .into_iter()
        .map(|c| c.equals(rf.related_field().m2m_columns()))
        .collect();
    let conditions_b: Vec<_> = b_columns
        .clone()
        .into_iter()
        .map(|c| c.equals(rf.m2m_columns()))
        .collect();

    // SELECT A_A.id, COUNT(*) AS <AGGREGATOR_ALIAS> FROM _AtoB
    // + INNER JOIN A AS A_A ON A_A.id = _AtoB.A
    // + INNER JOIN B AS B_B ON B_B.id = _AtoB.B
    let query = query
        .inner_join(
            model_a
                .as_table()
                .alias(model_a_alias)
                .on(ConditionTree::single(conditions_a)),
        )
        .inner_join(
            model_b
                .as_table()
                .alias(model_b_alias)
                .on(ConditionTree::single(conditions_b)),
        );

    // SELECT A_A.id, COUNT(*) AS <AGGREGATOR_ALIAS> FROM _AtoB
    // INNER JOIN A AS A_A ON A_A.id = _AtoB.A
    // INNER JOIN B AS B_B ON B_B.id = _AtoB.B
    // + GROUP BY A_A.id
    let query = a_columns.into_iter().fold(query, |acc, f| acc.group_by(f.clone()));

    let (left_fields, right_fields) = (a_ids.scalar_fields(), b_ids.scalar_fields());
    let pairs = left_fields.zip(right_fields);
    let on_conditions = pairs
        .map(|(a, b)| {
            let col_a = match previous_join {
                Some(prev_join) => Column::from((prev_join.alias.to_owned(), a.db_name().to_owned())),
                None => a.as_column(),
            };
            let col_b = Column::from((join_alias.to_owned(), b.db_name().to_owned()));

            col_a.equals(col_b)
        })
        .collect::<Vec<_>>();

    // + LEFT JOIN (
    //     SELECT A_A.id, COUNT(*) AS <AGGREGATOR_ALIAS> FROM _AtoB
    //       INNER JOIN A AS A_A ON (A_A.id = _AtoB.A)
    //       INNER JOIN B AS B_B ON (B_B.id = _AtoB.B)
    //     GROUP BY A.id
    // + ) AS <ORDER_JOIN_PREFIX> ON (<A | previous_join_alias>.id = <ORDER_JOIN_PREFIX>.id)
    let join = Table::from(query)
        .alias(join_alias.to_owned())
        .on(ConditionTree::single(on_conditions));

    AliasedJoin {
        alias: join_alias.to_owned(),
        data: join,
    }
}

pub fn compute_one2m_join(base_model: &ModelRef, rf: &RelationFieldRef, join_prefix: &str) -> AliasedJoin {
    let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
        (rf.scalar_fields(), rf.referenced_fields())
    } else {
        (
            rf.related_field().referenced_fields(),
            rf.related_field().scalar_fields(),
        )
    };

    // `rf` is always the relation field on the left model in the join (parent).
    let left_table_alias = if rf.model().name != base_model.name {
        Some(format!("{}_{}", join_prefix, &rf.model().name))
    } else {
        None
    };

    let right_table_alias = format!("{}_{}", join_prefix, &rf.related_model().name);

    let related_model = rf.related_model();
    let pairs = left_fields.into_iter().zip(right_fields.into_iter());

    let on_conditions = pairs
        .map(|(a, b)| {
            let a_col = if let Some(alias) = left_table_alias.clone() {
                Column::from((alias, a.db_name().to_owned()))
            } else {
                a.as_column()
            };

            let b_col = Column::from((right_table_alias.clone(), b.db_name().to_owned()));

            a_col.equals(b_col)
        })
        .collect::<Vec<_>>();

    AliasedJoin {
        alias: right_table_alias.to_owned(),
        data: related_model
            .as_table()
            .alias(right_table_alias)
            .on(ConditionTree::single(on_conditions)),
    }
}
