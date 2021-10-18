use connector_interface::filter::*;
use prisma_models::prelude::*;
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
    #[tracing::instrument(skip(self, alias))]
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
            Filter::Aggregation(filter) => filter.aliased_cond(alias),
            Filter::ScalarList(filter) => filter.aliased_cond(alias),
            Filter::Empty => ConditionTree::NoCondition,
        }
    }
}

impl AliasedCondition for ScalarFilter {
    /// Conversion from a `ScalarFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        match self.condition {
            ScalarCondition::Search(_, _) | ScalarCondition::NotSearch(_, _) => {
                scalar_filter_aliased_cond_search(self, alias)
            }
            _ => scalar_filter_aliased_cond(self, alias),
        }
    }
}

fn scalar_filter_aliased_cond_search(sf: ScalarFilter, alias: Option<Alias>) -> ConditionTree<'static> {
    let mut projections = match sf.condition.clone() {
        ScalarCondition::Search(_, proj) => proj,
        ScalarCondition::NotSearch(_, proj) => proj,
        _ => unreachable!(),
    };

    projections.push(sf.projection);

    let columns: Vec<Column> = projections
        .into_iter()
        .map(|p| match (p, alias) {
            (ScalarProjection::Single(field), None) => field.as_column(),
            (ScalarProjection::Single(field), Some(alias)) => field.as_column().table(alias.to_string(None)),
            (ScalarProjection::Compound(_), _) => unreachable!("Full-text search does not support compound fields"),
        })
        .collect();

    let comparable: Expression = text_search(columns.as_slice()).into();

    convert_scalar_filter(comparable, sf.condition, sf.mode, &[], false)
}

fn scalar_filter_aliased_cond(sf: ScalarFilter, alias: Option<Alias>) -> ConditionTree<'static> {
    match (alias, sf.projection) {
        (Some(alias), ScalarProjection::Single(field)) => {
            let comparable: Expression = field.as_column().table(alias.to_string(None)).into();

            convert_scalar_filter(comparable, sf.condition, sf.mode, &[field], false)
        }
        (Some(alias), ScalarProjection::Compound(fields)) => {
            let columns: Vec<Column<'static>> = fields
                .clone()
                .into_iter()
                .map(|field| field.as_column().table(alias.to_string(None)))
                .collect();

            convert_scalar_filter(Row::from(columns).into(), sf.condition, sf.mode, &fields, false)
        }
        (None, ScalarProjection::Single(field)) => {
            let comparable: Expression = field.as_column().into();

            convert_scalar_filter(comparable, sf.condition, sf.mode, &[field], false)
        }
        (None, ScalarProjection::Compound(fields)) => {
            let columns: Vec<Column<'static>> = fields.clone().into_iter().map(|field| field.as_column()).collect();

            convert_scalar_filter(Row::from(columns).into(), sf.condition, sf.mode, &fields, false)
        }
    }
}

impl AliasedCondition for ScalarListFilter {
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
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
    comparable: impl Comparable<'static>,
    cond: ScalarListCondition,
    field: &ScalarFieldRef,
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarListCondition::Contains(val) => {
            comparable.compare_raw("@>", Value::Array(Some(vec![convert_value(field, val)])))
        }
        ScalarListCondition::ContainsEvery(vals) => comparable.compare_raw("@>", convert_list_value(field, vals)),
        ScalarListCondition::ContainsSome(vals) => comparable.compare_raw("&&", convert_list_value(field, vals)),
        ScalarListCondition::IsEmpty(cond) if cond => comparable.compare_raw("=", Value::Array(Some(vec![])).raw()),
        ScalarListCondition::IsEmpty(_) => comparable.compare_raw("<>", Value::Array(Some(vec![])).raw()),
    };

    ConditionTree::single(condition)
}

impl AliasedCondition for RelationFilter {
    /// Conversion from a `RelationFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        let ids = self.field.model().primary_identifier().as_columns();
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

            let id_columns: Vec<Column<'static>> = self
                .field
                .linking_fields()
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
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        match self {
            AggregationFilter::Count(filter) => aggregate_conditions(*filter, alias, |x| count(x).into()),
            AggregationFilter::Average(filter) => aggregate_conditions(*filter, alias, |x| avg(x).into()),
            AggregationFilter::Sum(filter) => aggregate_conditions(*filter, alias, |x| sum(x).into()),
            AggregationFilter::Min(filter) => aggregate_conditions(*filter, alias, |x| min(x).into()),
            AggregationFilter::Max(filter) => aggregate_conditions(*filter, alias, |x| max(x).into()),
        }
    }
}

fn aggregate_conditions<T>(filter: Filter, alias: Option<Alias>, field_transformer: T) -> ConditionTree<'static>
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
            convert_scalar_filter(comparable, sf.condition, sf.mode, &[field], true)
        }
        (None, ScalarProjection::Single(field)) => {
            let comparable: Expression = field_transformer(field.as_column());
            convert_scalar_filter(comparable, sf.condition, sf.mode, &[field], true)
        }
    }
}

fn convert_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    mode: QueryMode,
    fields: &[ScalarFieldRef],
    is_parent_aggregation: bool,
) -> ConditionTree<'static> {
    match cond {
        ScalarCondition::JsonCompare(json_compare) => {
            convert_json_filter(comparable, json_compare, mode, fields.first().unwrap().to_owned())
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
    query_mode: QueryMode,
    field: ScalarFieldRef,
) -> ConditionTree<'static> {
    let json_filter_path = json_condition.path;
    let cond = json_condition.condition;
    let target_type = json_condition.target_type;
    let (expr_json, expr_string): (Expression, Expression) = if let Some(path) = json_filter_path {
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

    let condition: Expression = match *cond {
        ScalarCondition::Contains(value) => match target_type.unwrap() {
            JsonTargetType::String => expr_string
                .like(format!("{}", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            JsonTargetType::Array => expr_json.json_array_contains(convert_value(&field, value)).into(),
        },
        ScalarCondition::NotContains(value) => match target_type.unwrap() {
            JsonTargetType::String => expr_string
                .not_like(format!("{}", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            JsonTargetType::Array => expr_json.json_array_not_contains(convert_value(&field, value)).into(),
        },
        ScalarCondition::StartsWith(value) => match target_type.unwrap() {
            JsonTargetType::String => expr_string
                .begins_with(format!("{}", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            JsonTargetType::Array => expr_json
                .clone()
                .json_array_begins_with(convert_value(&field, value))
                .and(expr_json.json_type_equals(JsonType::Array))
                .into(),
        },
        ScalarCondition::NotStartsWith(value) => match target_type.unwrap() {
            JsonTargetType::String => expr_string
                .not_begins_with(format!("{}", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            JsonTargetType::Array => expr_json
                .clone()
                .json_array_not_begins_with(convert_value(&field, value))
                .and(expr_json.json_type_equals(JsonType::Array))
                .into(),
        },
        ScalarCondition::EndsWith(value) => match target_type.unwrap() {
            JsonTargetType::String => expr_string
                .ends_into(format!("{}", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            JsonTargetType::Array => expr_json
                .clone()
                .json_array_ends_into(convert_value(&field, value))
                .and(expr_json.json_type_equals(JsonType::Array))
                .into(),
        },
        ScalarCondition::NotEndsWith(value) => match target_type.unwrap() {
            JsonTargetType::String => expr_string
                .not_ends_into(format!("{}", value))
                .and(expr_json.json_type_equals(JsonType::String))
                .into(),
            JsonTargetType::Array => expr_json
                .clone()
                .json_array_not_ends_into(convert_value(&field, value))
                .and(expr_json.json_type_equals(JsonType::Array))
                .into(),
        },
        ScalarCondition::GreaterThan(value) => expr_json
            .clone()
            .greater_than(convert_value(&field, value.clone()))
            .and(filter_json_type(expr_json, value))
            .into(),
        ScalarCondition::GreaterThanOrEquals(value) => expr_json
            .clone()
            .greater_than_or_equals(convert_value(&field, value.clone()))
            .and(filter_json_type(expr_json, value))
            .into(),
        ScalarCondition::LessThan(value) => expr_json
            .clone()
            .less_than(convert_value(&field, value.clone()))
            .and(filter_json_type(expr_json, value))
            .into(),
        ScalarCondition::LessThanOrEquals(value) => expr_json
            .clone()
            .less_than_or_equals(convert_value(&field, value.clone()))
            .and(filter_json_type(expr_json, value))
            .into(),
        _ => {
            return convert_scalar_filter(expr_json, *cond, query_mode, &[field], false);
        }
    };

    ConditionTree::single(condition)
}

fn filter_json_type(comparable: Expression<'static>, value: PrismaValue) -> Compare {
    match value {
        PrismaValue::Json(json) => {
            let json: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();

            match json {
                serde_json::Value::String(_) => comparable.json_type_equals(JsonType::String),
                serde_json::Value::Number(_) => comparable.json_type_equals(JsonType::Number),
                v => panic!("JSON target types only accept strings or numbers, found: {}", v),
            }
        }
        _ => unreachable!(),
    }
}

fn default_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    fields: &[ScalarFieldRef],
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarCondition::Equals(PrismaValue::Null) => comparable.is_null(),
        ScalarCondition::NotEquals(PrismaValue::Null) => comparable.is_not_null(),
        ScalarCondition::Equals(value) => comparable.equals(convert_first_value(fields, value)),
        ScalarCondition::NotEquals(value) => comparable.not_equals(convert_first_value(fields, value)),
        ScalarCondition::Contains(value) => comparable.like(format!("{}", value)),
        ScalarCondition::NotContains(value) => comparable.not_like(format!("{}", value)),
        ScalarCondition::StartsWith(value) => comparable.begins_with(format!("{}", value)),
        ScalarCondition::NotStartsWith(value) => comparable.not_begins_with(format!("{}", value)),
        ScalarCondition::EndsWith(value) => comparable.ends_into(format!("{}", value)),
        ScalarCondition::NotEndsWith(value) => comparable.not_ends_into(format!("{}", value)),
        ScalarCondition::LessThan(value) => comparable.less_than(convert_first_value(fields, value)),
        ScalarCondition::LessThanOrEquals(value) => comparable.less_than_or_equals(convert_first_value(fields, value)),
        ScalarCondition::GreaterThan(value) => comparable.greater_than(convert_first_value(fields, value)),
        ScalarCondition::GreaterThanOrEquals(value) => {
            comparable.greater_than_or_equals(convert_first_value(fields, value))
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
        ScalarCondition::Search(value, _) => {
            let query: String = value
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.matches(query)
        }
        ScalarCondition::NotSearch(value, _) => {
            let query: String = value
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.not_matches(query)
        }
        ScalarCondition::JsonCompare(_) => unreachable!(),
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
        ScalarCondition::Equals(PrismaValue::Null) => comparable.is_null(),
        ScalarCondition::NotEquals(PrismaValue::Null) => comparable.is_not_null(),
        ScalarCondition::Equals(value) => comparable.compare_raw("ILIKE", format!("{}", value)),
        ScalarCondition::NotEquals(value) => comparable.compare_raw("NOT ILIKE", format!("{}", value)),
        ScalarCondition::Contains(value) => comparable.compare_raw("ILIKE", format!("%{}%", value)),
        ScalarCondition::NotContains(value) => comparable.compare_raw("NOT ILIKE", format!("%{}%", value)),
        ScalarCondition::StartsWith(value) => comparable.compare_raw("ILIKE", format!("{}%", value)),
        ScalarCondition::NotStartsWith(value) => comparable.compare_raw("NOT ILIKE", format!("{}%", value)),
        ScalarCondition::EndsWith(value) => comparable.compare_raw("ILIKE", format!("%{}", value)),
        ScalarCondition::NotEndsWith(value) => comparable.compare_raw("NOT ILIKE", format!("%{}", value)),
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
        ScalarCondition::In(values) => match values.split_first() {
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
        ScalarCondition::NotIn(values) => match values.split_first() {
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
        ScalarCondition::Search(value, _) => {
            let query: String = value
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.matches(query)
        }
        ScalarCondition::NotSearch(value, _) => {
            let query: String = value
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.not_matches(query)
        }
        ScalarCondition::JsonCompare(_) => unreachable!(),
    };

    ConditionTree::single(condition)
}

fn lower_if(expr: Expression<'static>, cond: bool) -> Expression<'static> {
    if cond {
        lower(expr).into()
    } else {
        expr
    }
}

fn convert_value<'a>(field: &ScalarFieldRef, value: PrismaValue) -> Value<'a> {
    field.value(value)
}

fn convert_first_value<'a>(fields: &[ScalarFieldRef], value: PrismaValue) -> Value<'a> {
    convert_value(fields.first().unwrap(), value)
}

fn convert_list_value<'a>(field: &ScalarFieldRef, values: Vec<PrismaValue>) -> Value<'a> {
    Value::Array(Some(values.into_iter().map(|val| convert_value(field, val)).collect()))
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
