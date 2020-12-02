use crate::{
    flavour::MssqlFlavour,
    pair::Pair,
    sql_migration::TableChange,
    sql_migration::{AddColumn, AlterColumn, DropColumn},
    sql_renderer::{common::IteratorJoin, SqlRenderer},
    sql_schema_differ::ColumnChanges,
};
use sql_schema_describer::walkers::{ColumnWalker, TableWalker};
use sql_schema_describer::DefaultValue;
use std::collections::BTreeSet;

/// Creates a set of `ALTER TABLE` statements in a correct execution order.
pub(crate) fn create_statements(
    renderer: &MssqlFlavour,
    tables: Pair<TableWalker<'_>>,
    changes: &[TableChange],
) -> Vec<String> {
    let constructor = AlterTableConstructor {
        renderer,
        tables,
        changes,
        drop_constraints: BTreeSet::new(),
        add_constraints: BTreeSet::new(),
        add_columns: Vec::new(),
        drop_columns: Vec::new(),
        column_mods: Vec::new(),
    };

    constructor.into_statements()
}

struct AlterTableConstructor<'a> {
    renderer: &'a MssqlFlavour,
    tables: Pair<TableWalker<'a>>,
    changes: &'a [TableChange],
    drop_constraints: BTreeSet<String>,
    add_constraints: BTreeSet<String>,
    add_columns: Vec<String>,
    drop_columns: Vec<String>,
    column_mods: Vec<String>,
}

impl<'a> AlterTableConstructor<'a> {
    fn into_statements(mut self) -> Vec<String> {
        for change in self.changes {
            match change {
                TableChange::DropPrimaryKey => {
                    self.drop_primary_key();
                }
                TableChange::AddPrimaryKey { columns } => {
                    self.add_primary_key(&columns);
                }
                TableChange::AddColumn(AddColumn { column_index }) => {
                    self.add_column(*column_index);
                }
                TableChange::DropColumn(DropColumn { index }) => {
                    self.drop_column(*index);
                }
                TableChange::DropAndRecreateColumn { column_index, .. } => {
                    self.drop_and_recreate_column(*column_index);
                }
                TableChange::AlterColumn(AlterColumn {
                    column_index,
                    changes,
                    type_change: _,
                }) => {
                    self.alter_column(*column_index, &changes);
                }
            };
        }

        // Order matters
        let mut statements = Vec::new();

        if !self.drop_constraints.is_empty() {
            statements.push(format!(
                "ALTER TABLE {} DROP CONSTRAINT {}",
                self.renderer.quote_with_schema(self.tables.previous().name()),
                self.drop_constraints.iter().join(",\n"),
            ));
        }

        if !self.column_mods.is_empty() {
            statements.extend(self.column_mods)
        }

        if !self.drop_columns.is_empty() {
            statements.push(format!(
                "ALTER TABLE {} DROP COLUMN {}",
                self.renderer.quote_with_schema(self.tables.previous().name()),
                self.drop_columns.join(",\n"),
            ));
        }

        if !self.add_constraints.is_empty() {
            statements.push(format!(
                "ALTER TABLE {} ADD {}",
                self.renderer.quote_with_schema(self.tables.previous().name()),
                self.add_constraints.iter().join(", ")
            ));
        }

        if !self.add_columns.is_empty() {
            statements.push(format!(
                "ALTER TABLE {} ADD {}",
                self.renderer.quote_with_schema(self.tables.previous().name()),
                self.add_columns.join(",\n"),
            ));
        }

        statements
    }

    fn drop_primary_key(&mut self) {
        let constraint = self
            .tables
            .previous()
            .primary_key()
            .and_then(|pk| pk.constraint_name.as_ref())
            .expect("Missing constraint name in DropPrimaryKey on MSSQL");

        self.drop_constraints
            .insert(format!("{}", self.renderer.quote(constraint)));
    }

    fn add_primary_key(&mut self, columns: &[String]) {
        let non_quoted_columns = columns.iter().map(|colname| colname);
        let mut quoted_columns = Vec::with_capacity(columns.len());

        for colname in columns {
            quoted_columns.push(format!("{}", self.renderer.quote(colname)));
        }

        self.add_constraints.insert(format!(
            "CONSTRAINT PK__{}__{} PRIMARY KEY ({})",
            self.tables.next().name(),
            non_quoted_columns.join("__"),
            quoted_columns.join(","),
        ));
    }

    fn add_column(&mut self, column_index: usize) {
        let column = self.tables.next().column_at(column_index);
        self.add_columns.push(self.renderer.render_column(&column));
    }

    fn drop_column(&mut self, column_index: usize) {
        let name = self
            .renderer
            .quote(self.tables.previous().column_at(column_index).name());

        self.drop_columns.push(format!("{}", name));
    }

    fn drop_and_recreate_column(&mut self, columns: Pair<usize>) {
        let columns = self.tables.columns(&columns);

        self.drop_columns
            .push(format!("{}", self.renderer.quote(columns.previous().name())));

        self.add_columns.push(self.renderer.render_column(columns.next()));
    }

    fn alter_column(&mut self, columns: Pair<usize>, changes: &ColumnChanges) {
        let columns = self.tables.columns(&columns);
        let expanded = expand_alter_column(&columns, changes);

        for alter in expanded.into_iter() {
            match alter {
                MsSqlAlterColumn::DropDefault { constraint_name } => {
                    let escaped = format!("{}", self.renderer.quote(&constraint_name));
                    self.drop_constraints.insert(escaped);
                }
                MsSqlAlterColumn::SetDefault(default) => {
                    let default = self
                        .renderer
                        .render_default(&default, &columns.next().column_type().family);

                    self.add_constraints.insert(format!(
                        "CONSTRAINT [DF__{table}__{column}] DEFAULT {default} FOR [{column}]",
                        table = self.tables.next().name(),
                        column = columns.next().name(),
                        default = default,
                    ));
                }
                MsSqlAlterColumn::Modify => {
                    let nullability = if columns.next().arity().is_required() {
                        "NOT NULL"
                    } else {
                        "NULL"
                    };

                    // If the column is part of a `UNIQUE` constraint, we must
                    // drop the constraint before we can drop the column.
                    let prev_constraints = self
                        .tables
                        .next()
                        .indexes()
                        .filter(|index| index.index_type().is_unique())
                        .filter(|index| index.contains_column(columns.previous().name()))
                        .collect::<Vec<_>>();

                    if !prev_constraints.is_empty() {
                        for constraint in prev_constraints {
                            self.drop_constraints
                                .insert(format!("{}", self.renderer.quote(constraint.name())));
                        }
                    }

                    self.column_mods.push(format!(
                        "ALTER TABLE {table} ALTER COLUMN {column_name} {column_type} {nullability}",
                        table = self.renderer.quote_with_schema(self.tables.previous().name()),
                        column_name = self.renderer.quote(&columns.next().name()),
                        column_type = super::render_column_type(columns.next()),
                        nullability = nullability,
                    ));

                    let next_constraints = self
                        .tables
                        .next()
                        .indexes()
                        .filter(|index| index.index_type().is_unique())
                        .filter(|index| index.contains_column(columns.previous().name()))
                        .collect::<Vec<_>>();

                    // Re-creating the `UNIQUE` constraint for the new column,
                    // if needed.
                    if !next_constraints.is_empty() {
                        for constraint in next_constraints {
                            let non_quoted_columns = constraint.column_names();

                            let quoted_columns = non_quoted_columns
                                .iter()
                                .map(|c| self.renderer.quote(c))
                                .map(|c| format!("{}", c))
                                .collect::<Vec<_>>();

                            self.add_constraints.insert(format!(
                                "CONSTRAINT PK__{}__{} UNIQUE ({})",
                                self.tables.next().name(),
                                non_quoted_columns.join("__"),
                                quoted_columns.join(","),
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
enum MsSqlAlterColumn {
    DropDefault { constraint_name: String },
    SetDefault(DefaultValue),
    Modify,
}

fn expand_alter_column(columns: &Pair<ColumnWalker<'_>>, column_changes: &ColumnChanges) -> Vec<MsSqlAlterColumn> {
    let mut changes = Vec::new();

    // Default value changes require us to re-create the constraint, which we
    // must do before modifying the column.
    if column_changes.default_changed() {
        if let Some(default) = columns.previous().default() {
            let constraint_name = default.constraint_name();

            changes.push(MsSqlAlterColumn::DropDefault {
                constraint_name: constraint_name.unwrap().into(),
            });
        }

        if !column_changes.only_default_changed() {
            changes.push(MsSqlAlterColumn::Modify);
        }

        if let Some(next_default) = columns.next().default() {
            changes.push(MsSqlAlterColumn::SetDefault(next_default.clone()));
        }
    } else {
        changes.push(MsSqlAlterColumn::Modify);
    }

    changes
}
