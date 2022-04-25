use super::render_default;
use crate::{
    flavour::MssqlFlavour,
    pair::Pair,
    sql_migration::AlterColumn,
    sql_migration::TableChange,
    sql_renderer::{
        common::{IteratorJoin, Quoted},
        SqlRenderer,
    },
    sql_schema_differ::ColumnChanges,
};
use sql_schema_describer::{
    mssql::MssqlSchemaExt,
    walkers::{ColumnWalker, TableWalker},
    ColumnId, DefaultValue,
};
use std::borrow::Cow;
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
        rename_primary_key: false,
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
    rename_primary_key: bool,
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
                TableChange::RenamePrimaryKey => {
                    self.rename_primary_key = true;
                }
                TableChange::AddPrimaryKey => {
                    self.add_primary_key();
                }
                TableChange::AddColumn {
                    column_id,
                    has_virtual_default: _,
                } => {
                    self.add_column(*column_id);
                }
                TableChange::DropColumn { column_id } => {
                    self.drop_column(*column_id);
                }
                TableChange::DropAndRecreateColumn { column_id, .. } => {
                    self.drop_and_recreate_column(*column_id);
                }
                TableChange::AlterColumn(AlterColumn {
                    column_id,
                    changes,
                    type_change: _,
                }) => {
                    self.alter_column(*column_id, changes);
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

        if self.rename_primary_key {
            let with_schema = format!(
                "{}.{}",
                self.renderer.schema_name(),
                self.tables
                    .previous()
                    .primary_key()
                    .unwrap()
                    .constraint_name
                    .as_ref()
                    .unwrap()
            );

            statements.push(format!(
                "EXEC SP_RENAME N{}, N{}",
                Quoted::Single(with_schema),
                Quoted::Single(
                    self.tables
                        .next()
                        .primary_key()
                        .unwrap()
                        .constraint_name
                        .as_ref()
                        .unwrap()
                ),
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
        let constraint_name = self
            .tables
            .previous()
            .primary_key()
            .and_then(|pk| pk.constraint_name.as_ref())
            .expect("Missing constraint name in DropPrimaryKey on MSSQL");

        self.drop_constraints
            .insert(format!("{}", self.renderer.quote(constraint_name)));
    }

    fn add_primary_key(&mut self) {
        let mssql_schema_ext: &MssqlSchemaExt = self.tables.next().schema().downcast_connector_data();
        let constraint_name = self
            .tables
            .next()
            .primary_key()
            .and_then(|pk| pk.constraint_name.as_ref())
            .expect("Missing constraint name in AddPrimaryKey on MSSQL");

        let columns = self.tables.next().primary_key_columns();
        let mut quoted_columns = Vec::with_capacity(columns.len());

        for column in columns {
            let mut rendered = format!("{}", self.renderer.quote(column.as_column().name()));

            if let Some(sort_order) = column.sort_order() {
                rendered.push(' ');
                rendered.push_str(sort_order.as_ref());
            }

            quoted_columns.push(rendered);
        }

        let clustering = if mssql_schema_ext.pk_is_clustered(self.tables.next().table_id()) {
            " CLUSTERED"
        } else {
            " NONCLUSTERED"
        };

        self.add_constraints.insert(format!(
            "CONSTRAINT {} PRIMARY KEY{} ({})",
            constraint_name,
            clustering,
            quoted_columns.join(","),
        ));
    }

    fn add_column(&mut self, column_id: ColumnId) {
        let column = self.tables.next().column_at(column_id);
        self.add_columns.push(self.renderer.render_column(&column));
    }

    fn drop_column(&mut self, column_id: ColumnId) {
        let name = self.renderer.quote(self.tables.previous().column_at(column_id).name());

        self.drop_columns.push(format!("{}", name));
    }

    fn drop_and_recreate_column(&mut self, columns: Pair<ColumnId>) {
        let columns = self.tables.columns(&columns);

        self.drop_columns
            .push(format!("{}", self.renderer.quote(columns.previous().name())));

        self.add_columns.push(self.renderer.render_column(columns.next()));
    }

    fn alter_column(&mut self, columns: Pair<ColumnId>, changes: &ColumnChanges) {
        let columns = self.tables.columns(&columns);
        let expanded = expand_alter_column(&columns, changes);

        for alter in expanded.into_iter() {
            match alter {
                MsSqlAlterColumn::DropDefault { constraint_name } => {
                    let escaped = format!("{}", self.renderer.quote(&constraint_name));
                    self.drop_constraints.insert(escaped);
                }
                MsSqlAlterColumn::SetDefault(default) => {
                    let constraint_name = default.constraint_name().map(Cow::from).unwrap_or_else(|| {
                        let old_default = format!("DF__{}__{}", self.tables.next().name(), columns.next().name());
                        Cow::from(old_default)
                    });

                    let default = render_default(&default);

                    self.add_constraints.insert(format!(
                        "CONSTRAINT [{constraint}] DEFAULT {default} FOR [{column}]",
                        constraint = constraint_name,
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

                    self.column_mods.push(format!(
                        "ALTER TABLE {table} ALTER COLUMN {column_name} {column_type} {nullability}",
                        table = self.renderer.quote_with_schema(self.tables.previous().name()),
                        column_name = self.renderer.quote(columns.next().name()),
                        column_type = super::render_column_type(columns.next()),
                        nullability = nullability,
                    ));
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
