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
            Filter::And(mut filters) => match filters.pop() {
                None => ConditionTree::NoCondition,
                Some(filter) => {
                    let right = filter.aliased_cond(alias);

                    filters.into_iter().rev().fold(right, |acc, filter| {
                        let left = filter.aliased_cond(alias);
                        ConditionTree::and(left, acc)
                    })
                }
            },
            Filter::Or(mut filters) => match filters.pop() {
                None => ConditionTree::NegativeCondition,
                Some(filter) => {
                    let right = filter.aliased_cond(alias);

                    filters.into_iter().rev().fold(right, |acc, filter| {
                        let left = filter.aliased_cond(alias);
                        ConditionTree::or(left, acc)
                    })
                }
            },
            Filter::Not(mut filters) => match filters.pop() {
                None => ConditionTree::NoCondition,
                Some(filter) => {
                    let right = filter.aliased_cond(alias).not();

                    filters.into_iter().rev().fold(right, |acc, filter| {
                        let left = filter.aliased_cond(alias).not();
                        ConditionTree::and(left, acc)
                    })
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
        let column = match alias {
            Some(ref alias) => self.field.as_column().table(alias.to_string(None)),
            None => self.field.as_column(),
        };

        let condition = match self.condition {
            ScalarCondition::Equals(PrismaValue::Null) => column.is_null(),
            ScalarCondition::NotEquals(PrismaValue::Null) => column.is_not_null(),
            ScalarCondition::Equals(value) => column.equals(value),
            ScalarCondition::NotEquals(value) => column.not_equals(value),
            ScalarCondition::Contains(value) => column.like(format!("{}", value)),
            ScalarCondition::NotContains(value) => column.not_like(format!("{}", value)),
            ScalarCondition::StartsWith(value) => column.begins_with(format!("{}", value)),
            ScalarCondition::NotStartsWith(value) => column.not_begins_with(format!("{}", value)),
            ScalarCondition::EndsWith(value) => column.ends_into(format!("{}", value)),
            ScalarCondition::NotEndsWith(value) => column.not_ends_into(format!("{}", value)),
            ScalarCondition::LessThan(value) => column.less_than(value),
            ScalarCondition::LessThanOrEquals(value) => column.less_than_or_equals(value),
            ScalarCondition::GreaterThan(value) => column.greater_than(value),
            ScalarCondition::GreaterThanOrEquals(value) => column.greater_than_or_equals(value),
            ScalarCondition::In(values) => column.in_selection(values),
            ScalarCondition::NotIn(values) => column.not_in_selection(values),
        };

        ConditionTree::single(condition)
    }
}

impl AliasedCondition for RelationFilter {
    /// Conversion from a `RelationFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        let identifier = self.field.model().identifier();
        let ids = identifier.as_columns();

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
    fn aliased_sel(self, alias: Option<Alias>) -> Select<'static> {
        let alias = alias.unwrap_or(Alias::default());
        let condition = self.condition.clone();
        let relation = self.field.relation();

        let these_columns = self
            .field
            .relation_columns(false)
            .map(|c| c.table(alias.to_string(None)));

        let other_columns = self.field.opposite_columns(false);
        let other_columns_len = other_columns.len();
        let other_columns = other_columns.map(|c| c.table(alias.to_string(None)));

        let id_columns = self.field.related_model().identifier().as_columns();
        let id_columns_len = id_columns.len();
        let id_columns = id_columns.map(|col| col.table(alias.to_string(Some(AliasMode::Join))));

        let related_table = self.field.related_model().as_table();
        let table = relation.as_table().alias(alias.to_string(Some(AliasMode::Table)));

        // check whether the join would join the same table and same column
        // example: `Track` AS `t1` INNER JOIN `Track` AS `j1` ON `j1`.`id` = `t1`.`id`
        let would_peform_needless_join = other_columns_len == id_columns_len
            && table.typ == related_table.typ
            && id_columns.zip(other_columns).all(|(id, other)| id == other);

        if would_peform_needless_join {
            // Don't do the useless join
            let conditions = self
                .nested_filter
                .aliased_cond(Some(alias))
                .invert_if(condition.invert_of_subselect());

            let select_base = Select::from_table(relation.as_table().alias(alias.to_string(None))).so_that(conditions);

            these_columns.fold(select_base, |acc, column| acc.column(column))
        } else {
            let other_columns: Vec<_> = self
                .field
                .opposite_columns(false)
                .map(|c| c.table(alias.to_string(None)))
                .collect();

            let identifiers: Vec<_> = self
                .field
                .related_model()
                .identifier()
                .as_columns()
                .map(|col| col.table(alias.to_string(Some(AliasMode::Join))))
                .collect();

            let conditions = self
                .nested_filter
                .aliased_cond(Some(alias.flip(AliasMode::Join)))
                .invert_if(condition.invert_of_subselect());

            let join = related_table
                .clone()
                .alias(alias.to_string(Some(AliasMode::Join)))
                .on(Row::from(identifiers).equals(Row::from(other_columns)));

            let select_base = Select::from_table(table).inner_join(join).so_that(conditions);

            these_columns.fold(select_base, |acc, column| acc.column(column))
        }
    }
}

impl AliasedCondition for OneRelationIsNullFilter {
    /// Conversion from a `OneRelationIsNullFilter` to a query condition tree. Aliased when in a nested `SELECT`.
    fn aliased_cond(self, alias: Option<Alias>) -> ConditionTree<'static> {
        let alias = alias.map(|a| a.to_string(None));

        let condition = if self.field.relation_is_inlined_in_parent() {
            self.field.as_column().opt_table(alias.clone()).is_null()
        } else {
            let relation = self.field.relation();

            let columns = relation
                .columns_for_relation_side(self.field.relation_side)
                .map(|c| c.opt_table(alias.clone()));

            let table = Table::from(relation.as_table());
            let relation_table = match alias {
                Some(ref alias) => table.alias(alias.to_string()),
                None => table,
            };

            let select = columns.fold(Select::from_table(relation_table), |acc, col| {
                acc.column(col.clone()).and_where(col.is_not_null())
            });

            let id_columns: Vec<Column<'static>> = self
                .field
                .model()
                .identifier()
                .as_columns()
                .map(|c| c.opt_table(alias.clone()))
                .collect();

            Row::from(id_columns).not_in_selection(select)
        };

        ConditionTree::single(condition)
    }
}
