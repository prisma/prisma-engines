use connector_interface::filter::*;
use prisma_models::prelude::*;
use quaint::ast::*;

#[derive(Clone, Copy, Debug)]
/// A distinction in aliasing to separate the parent table and the joined data
/// in the statement.
pub enum AliasMode {
    Table,
    Join,
}

impl Default for AliasMode {
    fn default() -> Self {
        AliasMode::Table
    }
}

#[derive(Clone, Copy, Debug, Default)]
/// Aliasing tool to count the nesting level to help with heavily nested
/// self-related queries.
pub struct Alias {
    counter: usize,
    mode: AliasMode,
}

impl Alias {
    /// Increment the alias as a new copy.
    ///
    /// Use when nesting one level down to a new subquery. `AliasMode` is
    /// required due to the fact the current mode can be in `AliasMode::Join`.
    pub fn inc(&self, mode: AliasMode) -> Self {
        Self {
            counter: self.counter + 1,
            mode,
        }
    }

    /// Flip the alias to a different mode keeping the same nesting count.
    pub fn flip(&self, mode: AliasMode) -> Self {
        Self {
            counter: self.counter,
            mode,
        }
    }

    /// A string representation of the current alias. The current mode can be
    /// overridden by defining the `mode_override`.
    pub fn to_string(&self, mode_override: Option<AliasMode>) -> String {
        match mode_override.unwrap_or(self.mode) {
            AliasMode::Table => format!("t{}", self.counter),
            AliasMode::Join => format!("j{}", self.counter),
        }
    }
}

pub trait AliasedCondition {
    /// Conversion to a query condition tree. Columns will point to the given
    /// alias if provided, otherwise using the fully qualified path.
    ///
    /// Alias should be used only when nesting, making the top level queries
    /// more explicit.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static>;
}

trait AliasedSelect {
    /// Conversion to a select. Columns will point to the given
    /// alias if provided, otherwise using the fully qualified path.
    ///
    /// Alias should be used only when nesting, making the top level queries
    /// more explicit.
    fn aliased_sel(self, alias: Option<Alias>) -> Select<'static>;
}

impl AliasedCondition for Filter {
    /// Conversion from a `Filter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        match self {
            Filter::And(mut filters) => match filters.len() {
                n if n == 0 => ConditionTree::NoCondition,
                n if n == 1 => filters.pop().unwrap().aliased_cond(alias),
                _ => {
                    let exprs = filters
                        .into_iter()
                        .map(|f| f.aliased_cond(alias))
                        .map(Expression::from)
                        .collect();

                    ConditionTree::And(exprs)
                }
            },
            Filter::Or(mut filters) => match filters.len() {
                n if n == 0 => ConditionTree::NegativeCondition,
                n if n == 1 => filters.pop().unwrap().aliased_cond(alias),
                _ => {
                    let exprs = filters
                        .into_iter()
                        .map(|f| f.aliased_cond(alias))
                        .map(Expression::from)
                        .collect();

                    ConditionTree::Or(exprs)
                }
            },
            Filter::Not(mut filters) => match filters.len() {
                n if n == 0 => ConditionTree::NoCondition,
                n if n == 1 => filters.pop().unwrap().aliased_cond(alias).not(),
                _ => {
                    let exprs = filters
                        .into_iter()
                        .map(|f| f.aliased_cond(alias).not())
                        .map(Expression::from)
                        .collect();

                    ConditionTree::And(exprs)
                }
            },
            Filter::Scalar(filter) => filter.aliased_cond(alias),
            Filter::OneRelationIsNull(filter) => filter.aliased_cond(alias),
            Filter::Relation(filter) => filter.aliased_cond(alias),
            Filter::BoolFilter(b) => {
                if b {
                    ConditionTree::NoCondition
                } else {
                    ConditionTree::NegativeCondition
                }
            }
            Filter::Empty => ConditionTree::NoCondition,
            _ => unimplemented!(),
        }
    }
}

impl AliasedCondition for ScalarFilter {
    /// Conversion from a `ScalarFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        match (alias, self.projection) {
            (Some(alias), ScalarProjection::Single(field)) => {
                let comparable: Expression = match self.mode {
                    QueryMode::Default => field.as_column().table(alias.to_string(None)).into(),
                    QueryMode::Insensitive => lower(field.as_column().table(alias.to_string(None))).into(),
                };

                convert_scalar_filter(comparable, self.condition, self.mode, &[field])
            }
            (Some(alias), ScalarProjection::Compound(fields)) => {
                let columns: Vec<Column<'static>> = fields
                    .clone()
                    .into_iter()
                    .map(|field| field.as_column().table(alias.to_string(None)))
                    .collect();

                convert_scalar_filter(Row::from(columns), self.condition, self.mode, &fields)
            }
            (None, ScalarProjection::Single(field)) => {
                let comparable: Expression = match self.mode {
                    QueryMode::Default => field.as_column().into(),
                    QueryMode::Insensitive => lower(field.as_column()).into(),
                };

                convert_scalar_filter(comparable, self.condition, self.mode, &[field])
            }
            (None, ScalarProjection::Compound(fields)) => {
                let columns: Vec<Column<'static>> = fields.clone().into_iter().map(|field| field.as_column()).collect();

                convert_scalar_filter(Row::from(columns), self.condition, self.mode, &fields)
            }
        }
    }
}

impl AliasedCondition for RelationFilter {
    /// Conversion from a `RelationFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        let ids = self.field.model().primary_identifier().as_columns();
        let columns: Vec<Column<'static>> = match alias {
            Some(alias) => ids.map(|c| c.table(alias.to_string(None))).collect(),
            None => ids.collect(),
        };

        let condition = self.condition.clone();
        let sub_select = self.aliased_sel(alias.map(|a| a.inc(AliasMode::Table)));

        let comparison = match condition {
            RelationCondition::AtLeastOneRelatedRecord => Row::from(columns).in_selection(sub_select),
            RelationCondition::EveryRelatedRecord => Row::from(columns).not_in_selection(sub_select),
            RelationCondition::NoRelatedRecord => Row::from(columns).not_in_selection(sub_select),
            RelationCondition::ToOneRelatedRecord => Row::from(columns).in_selection(sub_select),
        };

        comparison.into()
    }
}

impl AliasedSelect for RelationFilter {
    /// The subselect part of the `RelationFilter` `ConditionTree`.
    fn aliased_sel<'a>(self, alias: Option<Alias>) -> Select<'static> {
        let alias = alias.unwrap_or(Alias::default());
        let condition = self.condition.clone();

        let table = self.field.as_table();
        let selected_identifier: Vec<Column> = self
            .field
            .identifier_columns()
            .map(|c| c.table(alias.to_string(None)))
            .collect();

        let join_columns: Vec<Column> = self
            .field
            .join_columns()
            .map(|c| c.table(alias.to_string(None)))
            .collect();

        let related_table = self.field.related_model().as_table();
        let related_join_columns: Vec<_> = self
            .field
            .related_field()
            .linking_fields()
            .as_columns()
            .map(|col| col.table(alias.to_string(Some(AliasMode::Join))))
            .collect();

        let nested_conditions = self
            .nested_filter
            .aliased_cond(Some(alias.flip(AliasMode::Join)))
            .invert_if(condition.invert_of_subselect());

        let conditions = selected_identifier
            .clone()
            .into_iter()
            .fold(nested_conditions, |acc, column| acc.and(column.is_not_null()));

        let join = related_table
            .alias(alias.to_string(Some(AliasMode::Join)))
            .on(Row::from(related_join_columns).equals(Row::from(join_columns)));

        Select::from_table(table.alias(alias.to_string(Some(AliasMode::Table))))
            .columns(selected_identifier)
            .inner_join(join)
            .so_that(conditions)
    }
}

impl AliasedCondition for OneRelationIsNullFilter {
    /// Conversion from a `OneRelationIsNullFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        let alias = alias.map(|a| a.to_string(None));

        let condition = if self.field.relation_is_inlined_in_parent() {
            self.field.as_columns().fold(ConditionTree::NoCondition, |acc, column| {
                let column_is_null = column.opt_table(alias.clone()).is_null();

                match acc {
                    ConditionTree::NoCondition => column_is_null.into(),
                    cond => cond.and(column_is_null),
                }
            })
        } else {
            let relation = self.field.relation();
            let table = Table::from(relation.as_table());
            let relation_table = match alias {
                Some(ref alias) => table.alias(alias.to_string()),
                None => table,
            };

            let columns_not_null =
                self.field
                    .related_field()
                    .as_columns()
                    .fold(ConditionTree::NoCondition, |acc, column| {
                        let column_is_not_null = column.opt_table(alias.clone()).is_not_null();

                        match acc {
                            ConditionTree::NoCondition => column_is_not_null.into(),
                            cond => cond.and(column_is_not_null),
                        }
                    });

            let sub_select = Select::from_table(relation_table)
                .columns(self.field.related_field().as_columns())
                .and_where(columns_not_null);

            let id_columns: Vec<Column<'static>> = self
                .field
                .model()
                .primary_identifier()
                .as_columns()
                .map(|c| c.opt_table(alias.clone()))
                .collect();

            Row::from(id_columns).not_in_selection(sub_select).into()
        };

        ConditionTree::single(condition)
    }
}

fn convert_scalar_filter(
    comparable: impl Comparable<'static>,
    cond: ScalarCondition,
    mode: QueryMode,
    fields: &[ScalarFieldRef],
) -> ConditionTree<'static> {
    match mode {
        QueryMode::Default => default_scalar_filter(comparable, cond, fields),
        QueryMode::Insensitive => insensitive_scalar_filter(comparable, cond, fields),
    }
}

fn default_scalar_filter(
    comparable: impl Comparable<'static>,
    cond: ScalarCondition,
    fields: &[ScalarFieldRef],
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarCondition::Equals(PrismaValue::Null) => comparable.is_null(),
        ScalarCondition::NotEquals(PrismaValue::Null) => comparable.is_not_null(),
        ScalarCondition::Equals(value) => comparable.equals(convert_value(fields, value)),
        ScalarCondition::NotEquals(value) => comparable.not_equals(convert_value(fields, value)),
        ScalarCondition::Contains(value) => comparable.like(format!("{}", value)),
        ScalarCondition::NotContains(value) => comparable.not_like(format!("{}", value)),
        ScalarCondition::StartsWith(value) => comparable.begins_with(format!("{}", value)),
        ScalarCondition::NotStartsWith(value) => comparable.not_begins_with(format!("{}", value)),
        ScalarCondition::EndsWith(value) => comparable.ends_into(format!("{}", value)),
        ScalarCondition::NotEndsWith(value) => comparable.not_ends_into(format!("{}", value)),
        ScalarCondition::LessThan(value) => comparable.less_than(convert_value(fields, value)),
        ScalarCondition::LessThanOrEquals(value) => comparable.less_than_or_equals(convert_value(fields, value)),
        ScalarCondition::GreaterThan(value) => comparable.greater_than(convert_value(fields, value)),
        ScalarCondition::GreaterThanOrEquals(value) => comparable.greater_than_or_equals(convert_value(fields, value)),
        ScalarCondition::In(values) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_values(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                comparable.in_selection(sql_values)
            }
            _ => comparable.in_selection(convert_values(fields, values)),
        },
        ScalarCondition::NotIn(values) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_values(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                comparable.not_in_selection(sql_values)
            }
            _ => comparable.not_in_selection(convert_values(fields, values)),
        },
    };

    ConditionTree::single(condition)
}

fn insensitive_scalar_filter(
    comparable: impl Comparable<'static>,
    cond: ScalarCondition,
    fields: &[ScalarFieldRef],
) -> ConditionTree<'static> {
    // Current workaround: We assume we can use ILIKE when we see `mode: insensitive`, because postgres is the only DB that has
    // insensitive. We need a connector context for filter building that is unexpectedly complicated to integrate.
    let condition = match cond {
        ScalarCondition::Equals(PrismaValue::Null) => comparable.is_null(),
        ScalarCondition::NotEquals(PrismaValue::Null) => comparable.is_not_null(),
        ScalarCondition::Equals(value) => comparable.equals(lower(convert_value(fields, value))),
        ScalarCondition::NotEquals(value) => comparable.not_equals(convert_value(fields, value)),
        ScalarCondition::Contains(value) => comparable.compare_raw("ILIKE", format!("%{}%", value)),
        ScalarCondition::NotContains(value) => comparable.compare_raw("NOT ILIKE", format!("%{}%", value)),
        ScalarCondition::StartsWith(value) => comparable.compare_raw("ILIKE", format!("{}%", value)),
        ScalarCondition::NotStartsWith(value) => comparable.compare_raw("NOT ILIKE", format!("{}%", value)),
        ScalarCondition::EndsWith(value) => comparable.compare_raw("ILIKE", format!("%{}", value)),
        ScalarCondition::NotEndsWith(value) => comparable.compare_raw("NOT ILIKE", format!("%{}", value)),
        ScalarCondition::LessThan(value) => comparable.less_than(lower(convert_value(fields, value))),
        ScalarCondition::LessThanOrEquals(value) => comparable.less_than_or_equals(lower(convert_value(fields, value))),
        ScalarCondition::GreaterThan(value) => comparable.greater_than(lower(convert_value(fields, value))),
        ScalarCondition::GreaterThanOrEquals(value) => {
            comparable.greater_than_or_equals(lower(convert_value(fields, value)))
        }
        ScalarCondition::In(values) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_values(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                comparable.in_selection(sql_values)
            }
            _ => comparable.in_selection(
                values
                    .into_iter()
                    .map(|v| {
                        let val: Expression = lower(convert_value(fields, v)).into();
                        val
                    })
                    .collect::<Vec<_>>(),
            ),
        },
        ScalarCondition::NotIn(values) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_values(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                comparable.not_in_selection(sql_values)
            }
            _ => comparable.not_in_selection(
                values
                    .into_iter()
                    .map(|v| {
                        let val: Expression = lower(convert_value(fields, v)).into();
                        val
                    })
                    .collect::<Vec<_>>(),
            ),
        },
    };

    ConditionTree::single(condition)
}

fn convert_value<'a>(fields: &[ScalarFieldRef], value: PrismaValue) -> Value<'a> {
    fields.first().unwrap().value(value)
}

fn convert_values<'a>(fields: &[ScalarFieldRef], values: Vec<PrismaValue>) -> Vec<Value<'a>> {
    if fields.len() == values.len() {
        fields
            .into_iter()
            .zip(values)
            .map(|(field, value)| field.value(value))
            .collect()
    } else {
        let field = fields.first().unwrap();
        values.into_iter().map(|value| field.value(value)).collect()
    }
}
