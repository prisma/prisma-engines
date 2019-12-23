use sql_schema_describer::{Column, ColumnTypeFamily};

#[derive(Debug)]
pub(crate) struct ColumnDiffer<'a> {
    pub(crate) previous: &'a Column,
    pub(crate) next: &'a Column,
}

impl<'a> ColumnDiffer<'a> {
    pub(crate) fn differs_in_something(&self) -> bool {
        self.previous.name != self.next.name
            // TODO: compare the whole type
            // || self.previous.tpe != self.next.tpe
            || self.previous.tpe.family != self.next.tpe.family
            || self.previous.tpe.arity != self.next.tpe.arity
            || !self.defaults_match()
    }

    /// There are workarounds to cope with current migration and introspection limitations.
    ///
    /// - Since the values we set and introspect for timestamps are stringly typed, matching exactly the default value strings does not work on any database. Therefore we consider all datetime defaults as the same.
    ///
    /// - Postgres autoincrement fields get inferred with a default, which we want to ignore.
    ///
    /// - We want to consider string defaults with and without quotes the same, because they can be set without quotes and introspected back with quotes on sqlite and mariadb.
    fn defaults_match(&self) -> bool {
        if self.previous.auto_increment {
            return true;
        }

        if self.previous.tpe.family == ColumnTypeFamily::DateTime {
            return true;
        }

        let trimmed_previous = self.previous.default.as_ref().map(|s| s.trim_matches('\''));
        let trimmed_next = self.next.default.as_ref().map(|s| s.trim_matches('\''));

        trimmed_previous == trimmed_next
    }
}
