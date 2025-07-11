use crate::{Column, ForeignKeyId, IndexColumn, IndexId, SqlSchema, TableColumnId, TableId};

pub struct SqlSchemaFilter {
    /// Tables that shall be considered "externally" managed. As per prisma.config.ts > tables.external.
    /// Prisma will not consider those tables during diffing operations, migration creation, or introspection.
    /// They are still available for querying at runtime.
    pub external_tables: Vec<String>,
}

impl SqlSchemaFilter {
    /// Check if the given table name is in the list of external tables.
    /// `external_tables` can contain fully qualified table names with namespace
    /// (e.g. "auth.user") or just the table name.
    fn is_table_external(&self, namespace: Option<&str>, table_name: &str) -> bool {
        if let Some(namespace) = namespace {
            self.external_tables.contains(&format!("{}.{}", namespace, table_name))
                || self.external_tables.contains(&table_name.to_string())
        } else {
            self.external_tables.contains(&table_name.to_string())
        }
    }
}

/// Provides a mapping of old ids (pre filtering) to new ids (post filtering).
/// As the ids are index based, filtering out a table and its related items
/// changes the ids of every over element after the filtered item.
struct IdMapping {
    table_ids: Vec<Option<TableId>>,
    column_ids: Vec<Option<TableColumnId>>,
    index_ids: Vec<Option<IndexId>>,
    foreign_key_ids: Vec<Option<ForeignKeyId>>,
}

impl IdMapping {
    fn new(schema: &SqlSchema) -> Self {
        Self {
            table_ids: vec![None; schema.tables.len()],
            column_ids: vec![None; schema.table_columns.len()],
            index_ids: vec![None; schema.indexes.len()],
            foreign_key_ids: vec![None; schema.foreign_keys.len()],
        }
    }

    fn get_new_table_id(&mut self, old_table_id: TableId) -> Option<TableId> {
        self.table_ids[old_table_id.0 as usize]
    }

    fn get_new_column_id(&mut self, old_column_id: TableColumnId) -> Option<TableColumnId> {
        self.column_ids[old_column_id.0 as usize]
    }

    fn get_new_index_id(&mut self, old_index_id: IndexId) -> Option<IndexId> {
        self.index_ids[old_index_id.0 as usize]
    }

    fn get_new_foreign_key_id(&mut self, old_foreign_key_id: ForeignKeyId) -> Option<ForeignKeyId> {
        self.foreign_key_ids[old_foreign_key_id.0 as usize]
    }

    fn push_table_id(&mut self, old_table_id: TableId, new_table_id: TableId) {
        self.table_ids[old_table_id.0 as usize] = Some(new_table_id);
    }

    fn push_column_id(&mut self, old_column_id: TableColumnId, new_column_id: TableColumnId) {
        self.column_ids[old_column_id.0 as usize] = Some(new_column_id);
    }

    fn push_index_id(&mut self, old_index_id: IndexId, new_index_id: IndexId) {
        self.index_ids[old_index_id.0 as usize] = Some(new_index_id);
    }

    fn push_foreign_key_id(&mut self, old_foreign_key_id: ForeignKeyId, new_foreign_key_id: ForeignKeyId) {
        self.foreign_key_ids[old_foreign_key_id.0 as usize] = Some(new_foreign_key_id);
    }
}

impl SqlSchema {
    /// Consumes the existing schema and returns a filtered version of it.
    pub fn filter(self, filter: &SqlSchemaFilter) -> SqlSchema {
        if filter.external_tables.is_empty() {
            // Nothing to filter out => shortcut this
            return self;
        }

        let mut id_mapping = IdMapping::new(&self);
        let mut filtered_schema = SqlSchema::default();

        self.walk_namespaces().for_each(|namespace| {
            // Namespaces are filtered out on the database query level - not here.
            filtered_schema.push_namespace(namespace.name().to_string());
        });

        self.table_walkers().for_each(|table| {
            if filter.is_table_external(table.namespace(), table.name()) {
                return;
            }
            let new_table_id = filtered_schema.push_table(
                table.name().to_string(),
                table.namespace_id(),
                table.description().map(|d| d.to_string()),
            );
            id_mapping.push_table_id(table.id, new_table_id);
        });

        self.walk_table_columns().for_each(|column| {
            let new_table_id = id_mapping.get_new_table_id(column.table().id);
            if let Some(new_table_id) = new_table_id {
                let new_column_id = filtered_schema.push_table_column(
                    new_table_id,
                    Column {
                        name: column.name().to_string(),
                        tpe: column.column_type().clone(),
                        auto_increment: column.is_autoincrement(),
                        description: column.description().map(|d| d.to_string()),
                    },
                );
                id_mapping.push_column_id(column.id, new_column_id);
            }
        });

        self.walk_foreign_keys().for_each(|foreign_key| {
            let new_table_id = id_mapping.get_new_table_id(foreign_key.table().id);
            let new_referenced_table_id = id_mapping.get_new_table_id(foreign_key.referenced_table().id);
            if let (Some(new_table_id), Some(new_referenced_table_id)) = (new_table_id, new_referenced_table_id) {
                let new_foreign_key_id = filtered_schema.push_foreign_key(
                    foreign_key.constraint_name().map(|n| n.to_string()),
                    [new_table_id, new_referenced_table_id],
                    [foreign_key.on_delete_action(), foreign_key.on_update_action()],
                );
                id_mapping.push_foreign_key_id(foreign_key.id, new_foreign_key_id);
            }
        });

        self.table_default_values
            .iter()
            .for_each(|(table_column_id, default_value)| {
                let new_table_column_id = id_mapping.get_new_column_id(*table_column_id);
                if let Some(new_table_column_id) = new_table_column_id {
                    filtered_schema.push_table_default_value(new_table_column_id, default_value.clone());
                }
            });

        self.foreign_key_columns.iter().for_each(|foreign_key_column| {
            let new_foreign_key_id = id_mapping.get_new_foreign_key_id(foreign_key_column.foreign_key_id);
            let new_table_column_id = id_mapping.get_new_column_id(foreign_key_column.constrained_column);
            let new_referenced_table_column_id = id_mapping.get_new_column_id(foreign_key_column.referenced_column);
            if let (Some(new_foreign_key_id), Some(new_table_column_id), Some(new_referenced_table_column_id)) =
                (new_foreign_key_id, new_table_column_id, new_referenced_table_column_id)
            {
                filtered_schema.push_foreign_key_column(
                    new_foreign_key_id,
                    [new_table_column_id, new_referenced_table_column_id],
                );
            }
        });

        self.indexes.iter().enumerate().for_each(|(index_id, index)| {
            let new_table_id = id_mapping.get_new_table_id(index.table_id);
            if let Some(new_table_id) = new_table_id {
                let new_index_id =
                    filtered_schema.push_index_of_type(new_table_id, index.index_name.clone(), index.tpe);
                id_mapping.push_index_id(IndexId(index_id as u32), new_index_id);
            }
        });

        self.index_columns.iter().for_each(|index_column| {
            let new_index_id = id_mapping.get_new_index_id(index_column.index_id);
            let new_column_id = id_mapping.get_new_column_id(index_column.column_id);
            if let (Some(new_index_id), Some(new_column_id)) = (new_index_id, new_column_id) {
                filtered_schema.index_columns.push(IndexColumn {
                    index_id: new_index_id,
                    column_id: new_column_id,
                    sort_order: index_column.sort_order,
                    length: index_column.length,
                });
            }
        });

        self.check_constraints.iter().for_each(|(table_id, constraint_name)| {
            let new_table_id = id_mapping.get_new_table_id(*table_id);
            if let Some(new_table_id) = new_table_id {
                filtered_schema
                    .check_constraints
                    .push((new_table_id, constraint_name.clone()));
            }
        });

        // Rest of the schema does not have to be filtered
        filtered_schema.enums = self.enums;
        filtered_schema.enum_variants = self.enum_variants;
        filtered_schema.views = self.views;
        filtered_schema.view_columns = self.view_columns;
        filtered_schema.view_default_values = self.view_default_values;
        filtered_schema.procedures = self.procedures;
        filtered_schema.user_defined_types = self.user_defined_types;
        filtered_schema.connector_data = self.connector_data;

        filtered_schema
    }
}
