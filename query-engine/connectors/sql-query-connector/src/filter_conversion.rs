use crate::model_extensions::*;
use connector_interface::filter::*;
use prisma_models::prelude::*;
use quaint::ast::concat;
use quaint::ast::*;
use std::convert::TryInto;

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
    fn aliased_cond(self, alias: Option<Alias>, reverse: bool) -> ConditionTree<'static>;
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
    fn aliased_cond(self, alias: Option<Alias>, reverse: bool) -> ConditionTree<'static> {
        match self {
            Filter::And(mut filters) => match filters.len() {
                n if n == 0 => ConditionTree::NoCondition,
                n if n == 1 => filters.pop().unwrap().aliased_cond(alias, reverse),
                _ => {
                    let exprs = filters
                        .into_iter()
                        .map(|f| f.aliased_cond(alias, reverse))
                        .map(Expression::from)
                        .collect();

                    ConditionTree::And(exprs)
                }
            },
            Filter::Or(mut filters) => match filters.len() {
                n if n == 0 => ConditionTree::NegativeCondition,
                n if n == 1 => filters.pop().unwrap().aliased_cond(alias, reverse),
                _ => {
                    let exprs = filters
                        .into_iter()
                        .map(|f| f.aliased_cond(alias, reverse))
                        .map(Expression::from)
                        .collect();

                    ConditionTree::Or(exprs)
                }
            },
            Filter::Not(mut filters) => match filters.len() {
                n if n == 0 => ConditionTree::NoCondition,
                n if n == 1 => filters.pop().unwrap().aliased_cond(alias, !reverse).not(),
                _ => {
                    let exprs = filters
                        .into_iter()
                        .map(|f| f.aliased_cond(alias, !reverse).not())
                        .map(Expression::from)
                        .collect();

                    ConditionTree::And(exprs)
                }
            },
            Filter::Scalar(filter) => filter.aliased_cond(alias, reverse),
            Filter::OneRelationIsNull(filter) => filter.aliased_cond(alias, reverse),
            Filter::Relation(filter) => filter.aliased_cond(alias, reverse),
            Filter::BoolFilter(b) => {
                if b {
                    ConditionTree::NoCondition
                } else {
                    ConditionTree::NegativeCondition
                }
            }
            Filter::Aggregation(filter) => filter.aliased_cond(alias, reverse),
            Filter::ScalarList(filter) => filter.aliased_cond(alias, reverse),
            Filter::Empty => ConditionTree::NoCondition,
            Filter::Composite(_) => unimplemented!("SQL connectors do not support composites yet."),
        }
    }
}

impl AliasedCondition for ScalarFilter {
    /// Conversion from a `ScalarFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>, reverse: bool) -> ConditionTree<'static> {
        match self.condition {
            ScalarCondition::Search(_, _) | ScalarCondition::NotSearch(_, _) => {
                let mut projections = match self.condition.clone() {
                    ScalarCondition::Search(_, proj) => proj,
                    ScalarCondition::NotSearch(_, proj) => proj,
                    _ => unreachable!(),
                };

                projections.push(self.projection);

                let columns: Vec<Column> = projections
                    .into_iter()
                    .map(|p| match (p, alias) {
                        (ScalarProjection::Single(field), None) => field.as_column(),
                        (ScalarProjection::Single(field), Some(alias)) => {
                            field.as_column().table(alias.to_string(None))
                        }
                        (ScalarProjection::Compound(_), _) => {
                            unreachable!("Full-text search does not support compound fields")
                        }
                    })
                    .collect();

                let comparable: Expression = text_search(columns.as_slice()).into();

                convert_scalar_filter(comparable, self.condition, reverse, self.mode, &[], false)
            }
            _ => scalar_filter_aliased_cond(self, alias, reverse),
        }
    }
}

fn scalar_filter_aliased_cond(sf: ScalarFilter, alias: Option<Alias>, reverse: bool) -> ConditionTree<'static> {
    match (alias, sf.projection) {
        (Some(alias), ScalarProjection::Single(field)) => {
            let comparable: Expression = field.as_column().table(alias.to_string(None)).into();

            convert_scalar_filter(comparable, sf.condition, reverse, sf.mode, &[field], false)
        }
        (Some(alias), ScalarProjection::Compound(fields)) => {
            let columns: Vec<Column<'static>> = fields
                .clone()
                .into_iter()
                .map(|field| field.as_column().table(alias.to_string(None)))
                .collect();

            convert_scalar_filter(
                Row::from(columns).into(),
                sf.condition,
                reverse,
                sf.mode,
                &fields,
                false,
            )
        }
        (None, ScalarProjection::Single(field)) => {
            let comparable: Expression = field.as_column().into();

            convert_scalar_filter(comparable, sf.condition, reverse, sf.mode, &[field], false)
        }
        (None, ScalarProjection::Compound(fields)) => {
            let columns: Vec<Column<'static>> = fields.clone().into_iter().map(|field| field.as_column()).collect();

            convert_scalar_filter(
                Row::from(columns).into(),
                sf.condition,
                reverse,
                sf.mode,
                &fields,
                false,
            )
        }
    }
}

impl AliasedCondition for ScalarListFilter {
    fn aliased_cond(self, alias: Option<Alias>, _reverse: bool) -> ConditionTree<'static> {
        match alias {
            Some(alias) => {
                let comparable: Expression = self.field.as_column().table(alias.to_string(None)).into();
                convert_scalar_list_filter(comparable, self.condition, &self.field)
            }
            None => {
                let comparable: Expression = self.field.as_column().into();
                convert_scalar_list_filter(comparable, self.condition, &self.field)
            }
        }
    }
}

fn convert_scalar_list_filter(
    comparable: Expression<'static>,
    cond: ScalarListCondition,
    field: &ScalarFieldRef,
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarListCondition::Contains(ConditionValue::Value(val)) => {
            comparable.compare_raw("@>", convert_list_value(field, vec![val]))
        }
        ScalarListCondition::Contains(ConditionValue::FieldRef(field_ref)) => {
            let field_ref_expr: Expression = field_ref.as_column().into();

            // This code path is only reachable for connectors with `ScalarLists` capability
            field_ref_expr.equals(any_operator(comparable))
        }
        ScalarListCondition::ContainsEvery(ConditionListValue::List(vals)) => {
            comparable.compare_raw("@>", convert_list_value(field, vals))
        }
        ScalarListCondition::ContainsEvery(ConditionListValue::FieldRef(field_ref)) => {
            comparable.compare_raw("@>", field_ref.as_column())
        }
        ScalarListCondition::ContainsSome(ConditionListValue::List(vals)) => {
            comparable.compare_raw("&&", convert_list_value(field, vals))
        }
        ScalarListCondition::ContainsSome(ConditionListValue::FieldRef(field_ref)) => {
            comparable.compare_raw("&&", field_ref.as_column())
        }
        ScalarListCondition::IsEmpty(cond) if cond => comparable.compare_raw("=", Value::Array(Some(vec![])).raw()),
        ScalarListCondition::IsEmpty(_) => comparable.compare_raw("<>", Value::Array(Some(vec![])).raw()),
    };

    ConditionTree::single(condition)
}

impl AliasedCondition for RelationFilter {
    /// Conversion from a `RelationFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>, _reverse: bool) -> ConditionTree<'static> {
        let ids = ModelProjection::from(self.field.model().primary_identifier()).as_columns();
        let columns: Vec<Column<'static>> = match alias {
            Some(alias) => ids.map(|c| c.table(alias.to_string(None))).collect(),
            None => ids.collect(),
        };

        let condition = self.condition;
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
        let alias = alias.unwrap_or_default();
        let condition = self.condition;

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
        let related_join_columns: Vec<_> = ModelProjection::from(self.field.related_field().linking_fields())
            .as_columns()
            .map(|col| col.table(alias.to_string(Some(AliasMode::Join))))
            .collect();

        let nested_conditions = self
            .nested_filter
            .aliased_cond(Some(alias.flip(AliasMode::Join)), false)
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
    fn aliased_cond(self, alias: Option<Alias>, _reverse: bool) -> ConditionTree<'static> {
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
            let table = relation.as_table();
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

            // If the table is aliased, we need to use that alias in the SELECT too
            // eg: SELECT <alias>.x FROM table AS <alias>
            let columns: Vec<_> = self
                .field
                .related_field()
                .scalar_fields()
                .iter()
                .map(|f| match alias.as_ref() {
                    Some(a) => Column::from((a.clone(), f.db_name().to_owned())),
                    None => f.as_column(),
                })
                .collect();

            let sub_select = Select::from_table(relation_table)
                .columns(columns)
                .and_where(columns_not_null);

            let id_columns: Vec<Column<'static>> = ModelProjection::from(self.field.linking_fields())
                .as_columns()
                .map(|c| c.opt_table(alias.clone()))
                .collect();

            Row::from(id_columns).not_in_selection(sub_select).into()
        };

        ConditionTree::single(condition)
    }
}

impl AliasedCondition for AggregationFilter {
    /// Conversion from an `AggregationFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>, reverse: bool) -> ConditionTree<'static> {
        match self {
            AggregationFilter::Count(filter) => aggregate_conditions(*filter, alias, reverse, |x| count(x).into()),
            AggregationFilter::Average(filter) => aggregate_conditions(*filter, alias, reverse, |x| avg(x).into()),
            AggregationFilter::Sum(filter) => aggregate_conditions(*filter, alias, reverse, |x| sum(x).into()),
            AggregationFilter::Min(filter) => aggregate_conditions(*filter, alias, reverse, |x| min(x).into()),
            AggregationFilter::Max(filter) => aggregate_conditions(*filter, alias, reverse, |x| max(x).into()),
        }
    }
}

fn aggregate_conditions<T>(
    filter: Filter,
    alias: Option<Alias>,
    reverse: bool,
    field_transformer: T,
) -> ConditionTree<'static>
where
    T: Fn(Column) -> Expression,
{
    let sf = match filter {
        Filter::Scalar(sf) => sf,
        _ => unimplemented!(),
    };

    match (alias, sf.projection) {
        (_, ScalarProjection::Compound(_)) => {
            unimplemented!("Compound aggregate projections are unsupported.")
        }
        (Some(alias), ScalarProjection::Single(field)) => {
            let comparable: Expression = field_transformer(field.as_column().table(alias.to_string(None)));
            convert_scalar_filter(comparable, sf.condition, reverse, sf.mode, &[field], true)
        }
        (None, ScalarProjection::Single(field)) => {
            let comparable: Expression = field_transformer(field.as_column());
            convert_scalar_filter(comparable, sf.condition, reverse, sf.mode, &[field], true)
        }
    }
}

fn convert_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    reverse: bool,
    mode: QueryMode,
    fields: &[ScalarFieldRef],
    is_parent_aggregation: bool,
) -> ConditionTree<'static> {
    match cond {
        ScalarCondition::JsonCompare(json_compare) => {
            convert_json_filter(comparable, json_compare, reverse, fields.first().unwrap(), mode)
        }
        _ => match mode {
            QueryMode::Default => default_scalar_filter(comparable, cond, fields),
            QueryMode::Insensitive => insensitive_scalar_filter(comparable, cond, fields, is_parent_aggregation),
        },
    }
}

fn convert_json_filter(
    comparable: Expression<'static>,
    json_condition: JsonCondition,
    reverse: bool,
    field: &ScalarFieldRef,
    query_mode: QueryMode,
) -> ConditionTree<'static> {
    let JsonCondition {
        path,
        condition,
        target_type,
    } = json_condition;
    let (expr_json, expr_string): (Expression, Expression) = if let Some(path) = path {
        match path {
            JsonFilterPath::String(path) => (
                json_extract(comparable.clone(), JsonPath::string(path.clone()), false).into(),
                json_extract(comparable, JsonPath::string(path), true).into(),
            ),
            JsonFilterPath::Array(path) => (
                json_extract(comparable.clone(), JsonPath::array(path.clone()), false).into(),
                json_extract(comparable, JsonPath::array(path), true).into(),
            ),
        }
    } else {
        (comparable.clone(), comparable)
    };

    let condition: Expression = match *condition {
        ScalarCondition::Contains(value) => {
            (expr_json, expr_string).json_contains(field, value, target_type.unwrap(), reverse)
        }
        ScalarCondition::StartsWith(value) => {
            (expr_json, expr_string).json_starts_with(field, value, target_type.unwrap(), reverse)
        }
        ScalarCondition::EndsWith(value) => {
            (expr_json, expr_string).json_ends_with(field, value, target_type.unwrap(), reverse)
        }
        ScalarCondition::GreaterThan(value) => expr_json
            .clone()
            .greater_than(convert_value(field, value.clone()))
            .and(filter_json_type(expr_json, value))
            .into(),
        ScalarCondition::GreaterThanOrEquals(value) => expr_json
            .clone()
            .greater_than_or_equals(convert_value(field, value.clone()))
            .and(filter_json_type(expr_json, value))
            .into(),
        ScalarCondition::LessThan(value) => expr_json
            .clone()
            .less_than(convert_value(field, value.clone()))
            .and(filter_json_type(expr_json, value))
            .into(),
        ScalarCondition::LessThanOrEquals(value) => expr_json
            .clone()
            .less_than_or_equals(convert_value(field, value.clone()))
            .and(filter_json_type(expr_json, value))
            .into(),
        // Those conditions are unreachable because json filters are not accessible via the lowercase `not`.
        // They can only be inverted via the uppercase `NOT`, which doesn't invert filters but only renders a SQL `NOT`.
        ScalarCondition::NotContains(_) => unreachable!(),
        ScalarCondition::NotStartsWith(_) => unreachable!(),
        ScalarCondition::NotEndsWith(_) => unreachable!(),
        cond => {
            return convert_scalar_filter(expr_json, cond, reverse, query_mode, &[field.clone()], false);
        }
    };

    ConditionTree::single(condition)
}

fn filter_json_type(comparable: Expression<'static>, value: ConditionValue) -> Compare {
    match value {
        ConditionValue::Value(pv) => match pv {
            PrismaValue::Json(json) => {
                let json: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();

                match json {
                    serde_json::Value::String(_) => comparable.json_type_equals(JsonType::String),
                    serde_json::Value::Number(_) => comparable.json_type_equals(JsonType::Number),
                    v => panic!("JSON target types only accept strings or numbers, found: {}", v),
                }
            }
            _ => unreachable!(),
        },
        ConditionValue::FieldRef(field_ref) => comparable.json_type_equals(field_ref.as_column()),
    }
}

fn default_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    fields: &[ScalarFieldRef],
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarCondition::Equals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_null(),
        ScalarCondition::NotEquals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_not_null(),
        ScalarCondition::Equals(value) => comparable.equals(convert_first_value(fields, value)),
        ScalarCondition::NotEquals(value) => comparable.not_equals(convert_first_value(fields, value)),
        ScalarCondition::Contains(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("%{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                Value::text("%").raw().into(),
                field_ref.as_column().into(),
                Value::text("%").raw().into(),
            ])),
        },
        ScalarCondition::NotContains(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("%{}%", value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.as_column().into(),
                    Value::text("%").raw().into(),
                ]))
            }
        },
        ScalarCondition::StartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                field_ref.as_column().into(),
                Value::text("%").raw().into(),
            ])),
        },
        ScalarCondition::NotStartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("{}%", value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    field_ref.as_column().into(),
                    Value::text("%").raw().into(),
                ]))
            }
        },
        ScalarCondition::EndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("%{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                Value::text("%").raw().into(),
                field_ref.as_column().into(),
            ])),
        },
        ScalarCondition::NotEndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("%{}", value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.as_column().into(),
                ]))
            }
        },
        ScalarCondition::LessThan(value) => comparable.less_than(convert_first_value(fields, value)),
        ScalarCondition::LessThanOrEquals(value) => comparable.less_than_or_equals(convert_first_value(fields, value)),
        ScalarCondition::GreaterThan(value) => comparable.greater_than(convert_first_value(fields, value)),
        ScalarCondition::GreaterThanOrEquals(value) => {
            comparable.greater_than_or_equals(convert_first_value(fields, value))
        }
        ScalarCondition::In(ConditionListValue::List(values)) => match values.split_first() {
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
        ScalarCondition::In(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.equals(any_operator(field_ref.as_column()))
        }
        ScalarCondition::NotIn(ConditionListValue::List(values)) => match values.split_first() {
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
        ScalarCondition::NotIn(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.not_equals(all_operator(field_ref.as_column()))
        }
        ScalarCondition::Search(value, _) => {
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.matches(query)
        }
        ScalarCondition::NotSearch(value, _) => {
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.not_matches(query)
        }
        ScalarCondition::JsonCompare(_) => unreachable!(),
        ScalarCondition::IsSet(_) => unreachable!(),
    };

    ConditionTree::single(condition)
}

fn insensitive_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    fields: &[ScalarFieldRef],
    is_parent_aggregation: bool,
) -> ConditionTree<'static> {
    // Current workaround: We assume we can use ILIKE when we see `mode: insensitive`, because postgres is the only DB that has
    // insensitive. We need a connector context for filter building that is unexpectedly complicated to integrate.
    let condition = match cond {
        ScalarCondition::Equals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_null(),
        ScalarCondition::Equals(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw("ILIKE", field_ref.as_column()),
        },
        ScalarCondition::NotEquals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_not_null(),
        ScalarCondition::NotEquals(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw("NOT ILIKE", field_ref.as_column()),
        },
        ScalarCondition::Contains(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("%{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![
                    Value::text("%").into(),
                    field_ref.as_column().into(),
                    Value::text("%").into(),
                ]),
            ),
        },
        ScalarCondition::NotContains(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("%{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![
                    Value::text("%").into(),
                    field_ref.as_column().into(),
                    Value::text("%").into(),
                ]),
            ),
        },
        ScalarCondition::StartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![field_ref.as_column().into(), Value::text("%").into()]),
            ),
        },
        ScalarCondition::NotStartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![field_ref.as_column().into(), Value::text("%").into()]),
            ),
        },
        ScalarCondition::EndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("%{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![Value::text("%").into(), field_ref.as_column().into()]),
            ),
        },
        ScalarCondition::NotEndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("%{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![Value::text("%").into(), field_ref.as_column().into()]),
            ),
        },
        ScalarCondition::LessThan(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.less_than(lower(convert_first_value(fields, value)))
        }
        ScalarCondition::LessThanOrEquals(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.less_than_or_equals(lower(convert_first_value(fields, value)))
        }
        ScalarCondition::GreaterThan(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.greater_than(lower(convert_first_value(fields, value)))
        }
        ScalarCondition::GreaterThanOrEquals(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.greater_than_or_equals(lower(convert_first_value(fields, value)))
        }
        ScalarCondition::In(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_values(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

                comparable.in_selection(sql_values)
            }
            _ => {
                let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

                comparable.in_selection(
                    values
                        .into_iter()
                        .map(|v| {
                            let val: Expression = lower(convert_first_value(fields, v)).into();
                            val
                        })
                        .collect::<Vec<_>>(),
                )
            }
        },
        ScalarCondition::In(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.compare_raw("ILIKE", any_operator(field_ref.as_column()))
        }
        ScalarCondition::NotIn(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_values(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                let comparable: Expression = lower(comparable).into();

                comparable.not_in_selection(sql_values)
            }
            _ => {
                let comparable: Expression = lower(comparable).into();

                comparable.not_in_selection(
                    values
                        .into_iter()
                        .map(|v| {
                            let val: Expression = lower(convert_first_value(fields, v)).into();
                            val
                        })
                        .collect::<Vec<_>>(),
                )
            }
        },
        ScalarCondition::NotIn(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.compare_raw("NOT ILIKE", all_operator(field_ref.as_column()))
        }
        ScalarCondition::Search(value, _) => {
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.matches(query)
        }
        ScalarCondition::NotSearch(value, _) => {
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.not_matches(query)
        }
        ScalarCondition::JsonCompare(_) => unreachable!(),
        ScalarCondition::IsSet(_) => unreachable!(),
    };

    ConditionTree::single(condition)
}

fn lower_if<'a>(expr: Expression<'a>, cond: bool) -> Expression<'a> {
    if cond {
        lower(expr).into()
    } else {
        expr
    }
}

fn convert_value<'a>(field: &ScalarFieldRef, value: impl Into<ConditionValue>) -> Expression<'a> {
    match value.into() {
        ConditionValue::Value(pv) => field.value(pv).into(),
        ConditionValue::FieldRef(field_ref) => field_ref.as_column().into(),
    }
}

fn convert_first_value<'a>(fields: &[ScalarFieldRef], value: impl Into<ConditionValue>) -> Expression<'a> {
    match value.into() {
        ConditionValue::Value(pv) => convert_value(fields.first().unwrap(), pv).into(),
        ConditionValue::FieldRef(field_ref) => field_ref.as_column().into(),
    }
}

fn convert_list_value<'a>(field: &ScalarFieldRef, values: Vec<PrismaValue>) -> Expression<'a> {
    Value::Array(Some(values.into_iter().map(|val| field.value(val)).collect())).into()
}

fn convert_values<'a>(fields: &[ScalarFieldRef], values: Vec<PrismaValue>) -> Vec<Value<'a>> {
    if fields.len() == values.len() {
        fields
            .iter()
            .zip(values)
            .map(|(field, value)| field.value(value))
            .collect()
    } else {
        let field = fields.first().unwrap();
        values.into_iter().map(|value| field.value(value)).collect()
    }
}

trait JsonFilterExtension {
    fn json_contains(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
    ) -> Expression<'static>;

    fn json_starts_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
    ) -> Expression<'static>;

    fn json_ends_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
    ) -> Expression<'static>;
}

impl JsonFilterExtension for (Expression<'static>, Expression<'static>) {
    fn json_contains(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;

        match (value, target_type) {
            // string_contains (value)
            // TODO: .and should take `reverse` into account
            (ConditionValue::Value(value), JsonTargetType::String) => expr_string
                .like(format!("%{}%", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            // array_contains (value)
            // TODO: .and should take `reverse` into account
            (ConditionValue::Value(value), JsonTargetType::Array) => expr_json
                .clone()
                .json_array_contains(convert_value(field, value))
                .and(expr_json.json_type_equals(JsonType::Array))
                .into(),
            // string_contains (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let contains = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    json_unquote(field_ref.as_column()).into(),
                    Value::text("%").raw().into(),
                ]));

                if reverse {
                    contains
                        .or(expr_json.json_type_not_equals(JsonType::String))
                        .or(field_ref.as_column().json_type_not_equals(JsonType::String))
                        .into()
                } else {
                    contains
                        .and(expr_json.json_type_equals(JsonType::String))
                        .and(field_ref.as_column().json_type_equals(JsonType::String))
                        .into()
                }
            }
            // array_contains (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let contains = expr_json.clone().json_array_contains(convert_value(field, &field_ref));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
        }
    }

    fn json_starts_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;
        match (value, target_type) {
            // string_starts_with (value)
            // TODO: .and should take `reverse` into account
            (ConditionValue::Value(value), JsonTargetType::String) => expr_string
                .like(format!("{}%", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            // array_starts_with (value)
            // TODO: .and should take `reverse` into account
            (ConditionValue::Value(value), JsonTargetType::Array) => expr_json
                .clone()
                .json_array_begins_with(convert_value(field, value))
                .and(expr_json.json_type_equals(JsonType::Array))
                .into(),
            // string_starts_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let starts_with = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    json_unquote(field_ref.as_column()).into(),
                    Value::text("%").raw().into(),
                ]));

                if reverse {
                    starts_with
                        .or(expr_json.json_type_not_equals(JsonType::String))
                        .or(field_ref.as_column().json_type_not_equals(JsonType::String))
                        .into()
                } else {
                    starts_with
                        .and(expr_json.json_type_equals(JsonType::String))
                        .and(field_ref.as_column().json_type_equals(JsonType::String))
                        .into()
                }
            }
            // array_starts_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let starts_with = expr_json
                    .clone()
                    .json_array_begins_with(convert_value(field, &field_ref));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
        }
    }

    fn json_ends_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;

        match (value, target_type) {
            // string_ends_with (value)
            // TODO: .and should take `reverse` into account
            (ConditionValue::Value(value), JsonTargetType::String) => expr_string
                .like(format!("%{}", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            // array_ends_with (value)
            // TODO: .and should take `reverse` into account
            (ConditionValue::Value(value), JsonTargetType::Array) => expr_json
                .clone()
                .json_array_ends_into(convert_value(field, value))
                .and(expr_json.json_type_equals(JsonType::Array))
                .into(),
            // string_ends_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let ends_with = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    json_unquote(field_ref.as_column()).into(),
                ]));

                if reverse {
                    ends_with
                        .or(expr_json.json_type_not_equals(JsonType::String))
                        .or(field_ref.as_column().json_type_not_equals(JsonType::String))
                        .into()
                } else {
                    ends_with
                        .and(expr_json.json_type_equals(JsonType::String))
                        .and(field_ref.as_column().json_type_equals(JsonType::String))
                        .into()
                }
            }
            // array_ends_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let ends_with = expr_json.clone().json_array_ends_into(convert_value(field, &field_ref));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
        }
    }
}
