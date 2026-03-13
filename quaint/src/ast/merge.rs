use super::*;
use crate::error::*;
use std::convert::TryFrom;

/// A builder for SQL `MERGE` queries.
///
/// Not complete and not meant for external use in this state. Made for
/// compatibility purposes.
#[derive(Debug, PartialEq)]
pub struct Merge<'a> {
    pub(crate) table: Table<'a>,
    pub(crate) using: Using<'a>,
    pub(crate) when_matched: Option<Update<'a>>,
    pub(crate) when_not_matched: Option<Query<'a>>,
    pub(crate) returning: Option<Vec<Column<'a>>>,
}

impl<'a> Merge<'a> {
    pub(crate) fn new<T, U>(table: T, using: U) -> Self
    where
        T: Into<Table<'a>>,
        U: Into<Using<'a>>,
    {
        Self {
            table: table.into(),
            using: using.into(),
            when_matched: None,
            when_not_matched: None,
            returning: None,
        }
    }

    pub(crate) fn when_matched(mut self, update: Update<'a>) -> Self {
        self.when_matched = Some(update);
        self
    }

    pub(crate) fn when_not_matched<Q>(mut self, query: Q) -> Self
    where
        Q: Into<Query<'a>>,
    {
        self.when_not_matched = Some(query.into());
        self
    }

    pub(crate) fn returning<K, I>(mut self, columns: I) -> Self
    where
        K: Into<Column<'a>>,
        I: IntoIterator<Item = K>,
    {
        self.returning = Some(columns.into_iter().map(|k| k.into()).collect());
        self
    }

    /// Build a MERGE from an INSERT with `OnConflict::Update`.
    ///
    /// The ON condition is derived from the explicit constraint columns
    /// (not from `table.index_definitions`).
    pub(crate) fn from_insert_with_update(insert: Insert<'a>) -> crate::Result<Self> {
        let table = insert.table.ok_or_else(|| {
            let kind = ErrorKind::conversion("Insert needs to point to a table for conversion to Merge.");
            Error::builder(kind).build()
        })?;

        let (update, constraints) = match insert.on_conflict {
            Some(OnConflict::Update(update, constraints)) => (update, constraints),
            _ => {
                let kind = ErrorKind::conversion("Insert must have OnConflict::Update for this conversion.");
                return Err(Error::builder(kind).build());
            }
        };

        if constraints.is_empty() {
            let kind = ErrorKind::conversion("OnConflict::Update requires non-empty constraint columns.");
            return Err(Error::builder(kind).build());
        }

        let columns = insert.columns;

        for constraint in &constraints {
            if !columns.iter().any(|column| column.name == constraint.name) {
                let kind = ErrorKind::conversion(format!(
                    "OnConflict::Update constraint column `{}` must be present in the insert columns.",
                    constraint.name
                ));

                return Err(Error::builder(kind).build());
            }
        }

        let query = build_using_query(&columns, insert.values)?;
        let bare_columns: Vec<_> = columns.clone().into_iter().map(|c| c.into_bare()).collect();

        // Build ON conditions from the explicit constraint columns.
        let table_ref = match &table.typ {
            TableType::Table(name) => Table {
                typ: TableType::Table(name.clone()),
                alias: None,
                database: table.database.clone(),
                index_definitions: Vec::new(),
            },
            _ => {
                let kind = ErrorKind::conversion("Merge target must be a simple table.");
                return Err(Error::builder(kind).build());
            }
        };
        let on_conditions = build_on_conditions_from_constraints(&constraints, &table_ref);

        let using = query.into_using("dual", bare_columns.clone()).on(on_conditions);

        let dual_columns: Vec<_> = columns.into_iter().map(|c| c.table("dual")).collect();
        let not_matched = Insert::multi(bare_columns).values(dual_columns);
        let mut merge = Merge::new(table, using)
            .when_matched(update)
            .when_not_matched(not_matched);

        if let Some(columns) = insert.returning {
            merge = merge.returning(columns);
        }

        Ok(merge)
    }
}

/// Build ON conditions from explicit constraint columns (AND-joined).
fn build_on_conditions_from_constraints<'a>(constraints: &[Column<'a>], table: &Table<'a>) -> ConditionTree<'a> {
    let mut conditions: Option<ConditionTree<'a>> = None;

    for col in constraints {
        let bare_name = col.name.clone();
        let dual_col = Column::new(bare_name.clone()).table("dual");
        let table_col = Column::new(bare_name).table(table.clone());
        let cond = dual_col.equals(table_col);

        conditions = Some(match conditions {
            None => cond.into(),
            Some(existing) => existing.and(cond),
        });
    }

    conditions.unwrap_or(ConditionTree::NoCondition)
}

/// Extract the USING query from insert values — shared between DoNothing and Update paths.
fn build_using_query<'a>(columns: &[Column<'a>], values: Expression<'a>) -> crate::Result<Query<'a>> {
    match values.kind {
        ExpressionKind::Row(row) => {
            let cols_vals = columns.iter().zip(row.values);

            let select = cols_vals.fold(Select::default(), |query, (col, val)| {
                query.value(val.alias(col.name.clone()))
            });

            Ok(Query::from(select))
        }
        ExpressionKind::Values(values) => {
            let mut rows = values.rows.into_iter();
            let first_row = rows.next().ok_or_else(|| {
                let kind = ErrorKind::conversion("Insert values cannot be empty.");
                Error::builder(kind).build()
            })?;
            let cols_vals = columns.iter().zip(first_row.values);

            let select = cols_vals.fold(Select::default(), |query, (col, val)| {
                query.value(val.alias(col.name.clone()))
            });

            let union = rows.fold(Union::new(select), |union, row| {
                let cols_vals = columns.iter().zip(row.values);

                let select = cols_vals.fold(Select::default(), |query, (col, val)| {
                    query.value(val.alias(col.name.clone()))
                });

                union.all(select)
            });

            Ok(Query::from(union))
        }
        ExpressionKind::Selection(selection) => Ok(Query::from(selection)),
        ExpressionKind::Parameterized(value) => {
            Ok(Select::default().value(ExpressionKind::ParameterizedRow(value)).into())
        }
        _ => {
            let kind = ErrorKind::conversion("Insert type not supported.");
            Err(Error::builder(kind).build())
        }
    }
}

impl<'a> From<Merge<'a>> for Query<'a> {
    fn from(merge: Merge<'a>) -> Self {
        Self::Merge(Box::new(merge))
    }
}

#[derive(Debug, PartialEq)]
pub struct Using<'a> {
    pub(crate) base_query: Query<'a>,
    pub(crate) columns: Vec<Column<'a>>,
    pub(crate) as_table: Table<'a>,
    pub(crate) on_conditions: ConditionTree<'a>,
}

impl<'a> Using<'a> {
    pub(crate) fn on<T>(mut self, conditions: T) -> Self
    where
        T: Into<ConditionTree<'a>>,
    {
        self.on_conditions = conditions.into();
        self
    }
}

pub(crate) trait IntoUsing<'a> {
    fn into_using(self, alias: &'a str, columns: Vec<Column<'a>>) -> Using<'a>;
}

impl<'a, I> IntoUsing<'a> for I
where
    I: Into<Query<'a>>,
{
    fn into_using(self, alias: &'a str, columns: Vec<Column<'a>>) -> Using<'a> {
        Using {
            base_query: self.into(),
            as_table: Table::from(alias),
            columns,
            on_conditions: ConditionTree::NoCondition,
        }
    }
}

impl<'a> TryFrom<Insert<'a>> for Merge<'a> {
    type Error = Error;

    fn try_from(insert: Insert<'a>) -> crate::Result<Self> {
        let table = insert.table.ok_or_else(|| {
            let kind = ErrorKind::conversion("Insert needs to point to a table for conversion to Merge.");
            Error::builder(kind).build()
        })?;

        if table.index_definitions.is_empty() {
            let kind = ErrorKind::conversion("Insert table needs schema metadata for conversion to Merge.");
            return Err(Error::builder(kind).build());
        }

        let columns = insert.columns;
        let query = build_using_query(&columns, insert.values)?;

        let bare_columns: Vec<_> = columns.clone().into_iter().map(|c| c.into_bare()).collect();

        let using = query
            .into_using("dual", bare_columns.clone())
            .on(table.join_conditions(&columns)?);

        let dual_columns: Vec<_> = columns.into_iter().map(|c| c.table("dual")).collect();
        let not_matched = Insert::multi(bare_columns).values(dual_columns);
        let mut merge = Merge::new(table, using).when_not_matched(not_matched);

        if let Some(columns) = insert.returning {
            merge = merge.returning(columns);
        }

        Ok(merge)
    }
}
