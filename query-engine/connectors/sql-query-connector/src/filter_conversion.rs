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

trait AliasedColumn {
    /// Conversion to a column. Column will point to the given alias if provided, otherwise the fully qualified path.
    ///
    /// Alias should be used only when nesting, making the top level queries
    /// more explicit.
    fn aliased_col(self, alias: Option<Alias>) -> Column<'static>;
}

impl AliasedColumn for &ScalarFieldRef {
    fn aliased_col(self, alias: Option<Alias>) -> Column<'static> {
        self.as_column().aliased_col(alias)
    }
}

impl AliasedColumn for Column<'static> {
    fn aliased_col(self, alias: Option<Alias>) -> Column<'static> {
        match alias {
            Some(alias) => self.table(alias.to_string(None)),
            None => self,
        }
    }
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
                    .map(|p| match p {
                        ScalarProjection::Single(field) => field.aliased_col(alias),
                        ScalarProjection::Compound(_) => {
                            unreachable!("Full-text search does not support compound fields")
                        }
                    })
                    .collect();

                let comparable: Expression = text_search(columns.as_slice()).into();

                convert_scalar_filter(comparable, self.condition, reverse, self.mode, &[], alias, false)
            }
            _ => scalar_filter_aliased_cond(self, alias, reverse),
        }
    }
}

fn scalar_filter_aliased_cond(sf: ScalarFilter, alias: Option<Alias>, reverse: bool) -> ConditionTree<'static> {
    match sf.projection {
        ScalarProjection::Single(field) => {
            let comparable: Expression = field.aliased_col(alias).into();

            convert_scalar_filter(comparable, sf.condition, reverse, sf.mode, &[field], alias, false)
        }
        ScalarProjection::Compound(fields) => {
            let columns: Vec<Column<'static>> = fields
                .clone()
                .into_iter()
                .map(|field| field.aliased_col(alias))
                .collect();

            convert_scalar_filter(
                Row::from(columns).into(),
                sf.condition,
                reverse,
                sf.mode,
                &fields,
                alias,
                false,
            )
        }
    }
}

impl AliasedCondition for ScalarListFilter {
    fn aliased_cond(self, alias: Option<Alias>, _reverse: bool) -> ConditionTree<'static> {
        let comparable: Expression = self.field.aliased_col(alias).into();

        convert_scalar_list_filter(comparable, self.condition, &self.field, alias)
    }
}

fn convert_scalar_list_filter(
    comparable: Expression<'static>,
    cond: ScalarListCondition,
    field: &ScalarFieldRef,
    alias: Option<Alias>,
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarListCondition::Contains(ConditionValue::Value(val)) => {
            comparable.compare_raw("@>", convert_list_pv(field, vec![val]))
        }
        ScalarListCondition::Contains(ConditionValue::FieldRef(field_ref)) => {
            let field_ref_expr: Expression = field_ref.aliased_col(alias).into();

            // This code path is only reachable for connectors with `ScalarLists` capability
            field_ref_expr.equals(comparable.any())
        }
        ScalarListCondition::ContainsEvery(ConditionListValue::List(vals)) => {
            comparable.compare_raw("@>", convert_list_pv(field, vals))
        }
        ScalarListCondition::ContainsEvery(ConditionListValue::FieldRef(field_ref)) => {
            comparable.compare_raw("@>", field_ref.aliased_col(alias))
        }
        ScalarListCondition::ContainsSome(ConditionListValue::List(vals)) => {
            comparable.compare_raw("&&", convert_list_pv(field, vals))
        }
        ScalarListCondition::ContainsSome(ConditionListValue::FieldRef(field_ref)) => {
            comparable.compare_raw("&&", field_ref.aliased_col(alias))
        }
        ScalarListCondition::IsEmpty(true) => comparable.compare_raw("=", Value::Array(Some(vec![])).raw()),
        ScalarListCondition::IsEmpty(false) => comparable.compare_raw("<>", Value::Array(Some(vec![])).raw()),
    };

    ConditionTree::single(condition)
}

impl AliasedCondition for RelationFilter {
    /// Conversion from a `RelationFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>, _reverse: bool) -> ConditionTree<'static> {
        let ids = ModelProjection::from(self.field.model().primary_identifier()).as_columns();
        let columns: Vec<Column<'static>> = ids.map(|col| col.aliased_col(alias)).collect();

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
            .map(|col| col.aliased_col(Some(alias)))
            .collect();

        let join_columns: Vec<Column> = self.field.join_columns().map(|c| c.aliased_col(Some(alias))).collect();

        let related_table = self.field.related_model().as_table();
        let related_join_columns: Vec<_> = ModelProjection::from(self.field.related_field().linking_fields())
            .as_columns()
            .map(|col| col.aliased_col(Some(alias.flip(AliasMode::Join))))
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
                .map(|f| f.as_column().opt_table(alias.clone()))
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

    match sf.projection {
        ScalarProjection::Compound(_) => {
            unimplemented!("Compound aggregate projections are unsupported.")
        }
        ScalarProjection::Single(field) => {
            let comparable: Expression = field_transformer(field.aliased_col(alias));

            convert_scalar_filter(comparable, sf.condition, reverse, sf.mode, &[field], alias, true)
        }
    }
}

fn convert_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    reverse: bool,
    mode: QueryMode,
    fields: &[ScalarFieldRef],
    alias: Option<Alias>,
    is_parent_aggregation: bool,
) -> ConditionTree<'static> {
    match cond {
        ScalarCondition::JsonCompare(json_compare) => {
            convert_json_filter(comparable, json_compare, reverse, fields.first().unwrap(), mode, alias)
        }
        _ => match mode {
            QueryMode::Default => default_scalar_filter(comparable, cond, fields, alias),
            QueryMode::Insensitive => insensitive_scalar_filter(comparable, cond, fields, alias, is_parent_aggregation),
        },
    }
}

fn convert_json_filter(
    comparable: Expression<'static>,
    json_condition: JsonCondition,
    reverse: bool,
    field: &ScalarFieldRef,
    query_mode: QueryMode,
    alias: Option<Alias>,
) -> ConditionTree<'static> {
    let JsonCondition {
        path,
        condition,
        target_type,
    } = json_condition;
    let (expr_json, expr_string): (Expression, Expression) = match path {
        Some(JsonFilterPath::String(path)) => (
            json_extract(comparable.clone(), JsonPath::string(path.clone()), false).into(),
            json_extract(comparable, JsonPath::string(path), true).into(),
        ),
        Some(JsonFilterPath::Array(path)) => (
            json_extract(comparable.clone(), JsonPath::array(path.clone()), false).into(),
            json_extract(comparable, JsonPath::array(path), true).into(),
        ),
        _ => (comparable.clone(), comparable),
    };

    let condition: Expression = match *condition {
        ScalarCondition::Contains(value) => {
            (expr_json, expr_string).json_contains(field, value, target_type.unwrap(), reverse, alias)
        }
        ScalarCondition::StartsWith(value) => {
            (expr_json, expr_string).json_starts_with(field, value, target_type.unwrap(), reverse, alias)
        }
        ScalarCondition::EndsWith(value) => {
            (expr_json, expr_string).json_ends_with(field, value, target_type.unwrap(), reverse, alias)
        }
        ScalarCondition::GreaterThan(value) => {
            let gt = expr_json
                .clone()
                .greater_than(convert_value(field, value.clone(), alias));

            with_json_type_filter(gt, expr_json, value, alias, reverse)
        }
        ScalarCondition::GreaterThanOrEquals(value) => {
            let gte = expr_json
                .clone()
                .greater_than_or_equals(convert_value(field, value.clone(), alias));

            with_json_type_filter(gte, expr_json, value, alias, reverse)
        }
        ScalarCondition::LessThan(value) => {
            let lt = expr_json.clone().less_than(convert_value(field, value.clone(), alias));

            with_json_type_filter(lt, expr_json, value, alias, reverse)
        }
        ScalarCondition::LessThanOrEquals(value) => {
            let lte = expr_json
                .clone()
                .less_than_or_equals(convert_value(field, value.clone(), alias));

            with_json_type_filter(lte, expr_json, value, alias, reverse)
        }
        // Those conditions are unreachable because json filters are not accessible via the lowercase `not`.
        // They can only be inverted via the uppercase `NOT`, which doesn't invert filters but adds a Filter::Not().
        ScalarCondition::NotContains(_) => unreachable!(),
        ScalarCondition::NotStartsWith(_) => unreachable!(),
        ScalarCondition::NotEndsWith(_) => unreachable!(),
        cond => {
            return convert_scalar_filter(expr_json, cond, reverse, query_mode, &[field.clone()], alias, false);
        }
    };

    ConditionTree::single(condition)
}

fn with_json_type_filter(
    comparable: Compare<'static>,
    expr_json: Expression<'static>,
    value: ConditionValue,
    alias: Option<Alias>,
    reverse: bool,
) -> Expression<'static> {
    match value {
        ConditionValue::Value(pv) => match pv {
            PrismaValue::Json(json) => {
                let json: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();

                match json {
                    serde_json::Value::String(_) if reverse => {
                        comparable.or(expr_json.json_type_not_equals(JsonType::String)).into()
                    }
                    serde_json::Value::String(_) => comparable.and(expr_json.json_type_equals(JsonType::String)).into(),
                    serde_json::Value::Number(_) if reverse => {
                        comparable.or(expr_json.json_type_not_equals(JsonType::Number)).into()
                    }
                    serde_json::Value::Number(_) => comparable.and(expr_json.json_type_equals(JsonType::Number)).into(),
                    v => panic!("JSON target types only accept strings or numbers, found: {}", v),
                }
            }
            _ => unreachable!(),
        },
        ConditionValue::FieldRef(field_ref) if reverse => comparable
            .or(expr_json.json_type_not_equals(field_ref.aliased_col(alias)))
            .into(),
        ConditionValue::FieldRef(field_ref) => comparable
            .and(expr_json.json_type_equals(field_ref.aliased_col(alias)))
            .into(),
    }
}

fn default_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    fields: &[ScalarFieldRef],
    alias: Option<Alias>,
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarCondition::Equals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_null(),
        ScalarCondition::NotEquals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_not_null(),
        ScalarCondition::Equals(value) => comparable.equals(convert_first_value(fields, value, alias)),
        ScalarCondition::NotEquals(value) => comparable.not_equals(convert_first_value(fields, value, alias)),
        ScalarCondition::Contains(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("%{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                Value::text("%").raw().into(),
                field_ref.aliased_col(alias).into(),
                Value::text("%").raw().into(),
            ])),
        },
        ScalarCondition::NotContains(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("%{}%", value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias).into(),
                    Value::text("%").raw().into(),
                ]))
            }
        },
        ScalarCondition::StartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                field_ref.aliased_col(alias).into(),
                Value::text("%").raw().into(),
            ])),
        },
        ScalarCondition::NotStartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("{}%", value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    field_ref.aliased_col(alias).into(),
                    Value::text("%").raw().into(),
                ]))
            }
        },
        ScalarCondition::EndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("%{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                Value::text("%").raw().into(),
                field_ref.aliased_col(alias).into(),
            ])),
        },
        ScalarCondition::NotEndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("%{}", value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias).into(),
                ]))
            }
        },
        ScalarCondition::LessThan(value) => comparable.less_than(convert_first_value(fields, value, alias)),
        ScalarCondition::LessThanOrEquals(value) => {
            comparable.less_than_or_equals(convert_first_value(fields, value, alias))
        }
        ScalarCondition::GreaterThan(value) => comparable.greater_than(convert_first_value(fields, value, alias)),
        ScalarCondition::GreaterThanOrEquals(value) => {
            comparable.greater_than_or_equals(convert_first_value(fields, value, alias))
        }
        ScalarCondition::In(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                comparable.in_selection(sql_values)
            }
            _ => comparable.in_selection(convert_pvs(fields, values)),
        },
        ScalarCondition::In(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.equals(Expression::from(field_ref.aliased_col(alias)).any())
        }
        ScalarCondition::NotIn(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                comparable.not_in_selection(sql_values)
            }
            _ => comparable.not_in_selection(convert_pvs(fields, values)),
        },
        ScalarCondition::NotIn(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.not_equals(Expression::from(field_ref.aliased_col(alias)).all())
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
    alias: Option<Alias>,
    is_parent_aggregation: bool,
) -> ConditionTree<'static> {
    // Current workaround: We assume we can use ILIKE when we see `mode: insensitive`, because postgres is the only DB that has
    // insensitive. We need a connector context for filter building that is unexpectedly complicated to integrate.
    let condition = match cond {
        ScalarCondition::Equals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_null(),
        ScalarCondition::Equals(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw("ILIKE", field_ref.aliased_col(alias)),
        },
        ScalarCondition::NotEquals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_not_null(),
        ScalarCondition::NotEquals(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw("NOT ILIKE", field_ref.aliased_col(alias)),
        },
        ScalarCondition::Contains(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("%{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![
                    Value::text("%").into(),
                    field_ref.aliased_col(alias).into(),
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
                    field_ref.aliased_col(alias).into(),
                    Value::text("%").into(),
                ]),
            ),
        },
        ScalarCondition::StartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![field_ref.aliased_col(alias).into(), Value::text("%").into()]),
            ),
        },
        ScalarCondition::NotStartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("{}%", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![field_ref.aliased_col(alias).into(), Value::text("%").into()]),
            ),
        },
        ScalarCondition::EndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("%{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![Value::text("%").into(), field_ref.aliased_col(alias).into()]),
            ),
        },
        ScalarCondition::NotEndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("%{}", value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![Value::text("%").into(), field_ref.aliased_col(alias).into()]),
            ),
        },
        ScalarCondition::LessThan(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.less_than(lower(convert_first_value(fields, value, alias)))
        }
        ScalarCondition::LessThanOrEquals(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.less_than_or_equals(lower(convert_first_value(fields, value, alias)))
        }
        ScalarCondition::GreaterThan(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.greater_than(lower(convert_first_value(fields, value, alias)))
        }
        ScalarCondition::GreaterThanOrEquals(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.greater_than_or_equals(lower(convert_first_value(fields, value, alias)))
        }
        ScalarCondition::In(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap());
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
                        .map(|value| {
                            let val: Expression = lower(convert_first_value(fields, value, alias)).into();
                            val
                        })
                        .collect::<Vec<_>>(),
                )
            }
        },
        ScalarCondition::In(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.compare_raw("ILIKE", Expression::from(field_ref.aliased_col(alias)).any())
        }
        ScalarCondition::NotIn(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap());
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
                        .map(|value| {
                            let val: Expression = lower(convert_first_value(fields, value, alias)).into();
                            val
                        })
                        .collect::<Vec<_>>(),
                )
            }
        },
        ScalarCondition::NotIn(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.compare_raw("NOT ILIKE", Expression::from(field_ref.aliased_col(alias)).all())
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

fn lower_if(expr: Expression<'_>, cond: bool) -> Expression<'_> {
    if cond {
        lower(expr).into()
    } else {
        expr
    }
}

fn convert_value<'a>(field: &ScalarFieldRef, value: impl Into<ConditionValue>, alias: Option<Alias>) -> Expression<'a> {
    match value.into() {
        ConditionValue::Value(pv) => convert_pv(field, pv),
        ConditionValue::FieldRef(field_ref) => field_ref.aliased_col(alias).into(),
    }
}

fn convert_first_value<'a>(
    fields: &[ScalarFieldRef],
    value: impl Into<ConditionValue>,
    alias: Option<Alias>,
) -> Expression<'a> {
    match value.into() {
        ConditionValue::Value(pv) => convert_pv(fields.first().unwrap(), pv),
        ConditionValue::FieldRef(field_ref) => field_ref.aliased_col(alias).into(),
    }
}

fn convert_pv<'a>(field: &ScalarFieldRef, pv: PrismaValue) -> Expression<'a> {
    field.value(pv).into()
}

fn convert_list_pv<'a>(field: &ScalarFieldRef, values: Vec<PrismaValue>) -> Expression<'a> {
    Value::Array(Some(values.into_iter().map(|val| field.value(val)).collect())).into()
}

fn convert_pvs<'a>(fields: &[ScalarFieldRef], values: Vec<PrismaValue>) -> Vec<Value<'a>> {
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

trait JsonFilterExt {
    fn json_contains(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
    ) -> Expression<'static>;

    fn json_starts_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
    ) -> Expression<'static>;

    fn json_ends_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
    ) -> Expression<'static>;
}

impl JsonFilterExt for (Expression<'static>, Expression<'static>) {
    fn json_contains(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;

        match (value, target_type) {
            // string_contains (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let contains = expr_string.like(format!("%{}%", value));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_contains (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                let contains = expr_json.clone().json_array_contains(convert_pv(field, value));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_contains (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let contains = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias).into(),
                    Value::text("%").raw().into(),
                ]));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_contains (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let contains = expr_json.clone().json_array_contains(field_ref.aliased_col(alias));

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
        alias: Option<Alias>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;
        match (value, target_type) {
            // string_starts_with (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let starts_with = expr_string.like(format!("{}%", value));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_starts_with (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                let starts_with = expr_json.clone().json_array_begins_with(convert_pv(field, value));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_starts_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let starts_with = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    field_ref.aliased_col(alias).into(),
                    Value::text("%").raw().into(),
                ]));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_starts_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let starts_with = expr_json.clone().json_array_begins_with(field_ref.aliased_col(alias));

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
        alias: Option<Alias>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;

        match (value, target_type) {
            // string_ends_with (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let ends_with = expr_string.like(format!("%{}", value));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_ends_with (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                let ends_with = expr_json.clone().json_array_ends_into(convert_pv(field, value));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_ends_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let ends_with = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias).into(),
                ]));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_ends_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let ends_with = expr_json.clone().json_array_ends_into(field_ref.aliased_col(alias));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
        }
    }
}
