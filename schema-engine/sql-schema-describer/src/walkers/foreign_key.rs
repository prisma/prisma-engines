use crate::{ForeignKey, ForeignKeyAction, ForeignKeyColumn, ForeignKeyId, TableColumnWalker, TableWalker, Walker};

/// Traverse a foreign key.
pub type ForeignKeyWalker<'a> = Walker<'a, ForeignKeyId>;

impl<'schema> ForeignKeyWalker<'schema> {
    pub(super) fn columns(self) -> &'schema [ForeignKeyColumn] {
        let range = super::range_for_key(&self.schema.foreign_key_columns, self.id, |col| col.foreign_key_id);
        &self.schema.foreign_key_columns[range]
    }

    /// The foreign key columns on the referencing table.
    pub fn constrained_columns(self) -> impl ExactSizeIterator<Item = TableColumnWalker<'schema>> {
        self.columns().iter().map(move |col| self.walk(col.constrained_column))
    }

    /// The name of the foreign key constraint.
    pub fn constraint_name(self) -> Option<&'schema str> {
        self.foreign_key().constraint_name.as_deref()
    }

    fn foreign_key(self) -> &'schema ForeignKey {
        &self.schema.foreign_keys[self.id.0 as usize]
    }

    /// The `ON DELETE` behaviour of the foreign key.
    pub fn on_delete_action(self) -> ForeignKeyAction {
        self.foreign_key().on_delete_action
    }

    /// The `ON UPDATE` behaviour of the foreign key.
    pub fn on_update_action(self) -> ForeignKeyAction {
        self.foreign_key().on_update_action
    }

    /// The columns referenced by the foreign key on the referenced table.
    pub fn referenced_columns(self) -> impl ExactSizeIterator<Item = TableColumnWalker<'schema>> {
        self.columns().iter().map(move |col| self.walk(col.referenced_column))
    }

    /// The table the foreign key "points to".
    pub fn referenced_table_name(self) -> &'schema str {
        self.referenced_table().name()
    }

    /// The table the foreign key "points to".
    pub fn referenced_table(self) -> TableWalker<'schema> {
        self.walk(self.foreign_key().referenced_table)
    }

    /// Traverse to the referencing/constrained table.
    pub fn table(self) -> TableWalker<'schema> {
        self.walk(self.foreign_key().constrained_table)
    }

    /// True if relation is back to the same table.
    pub fn is_self_relation(self) -> bool {
        let fk = self.foreign_key();
        fk.constrained_table == fk.referenced_table
    }
}
