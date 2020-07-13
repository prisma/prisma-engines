use crate::sql_schema_helpers::ColumnRef;
use prisma_models::PrismaValue;
use sql_schema_describer::{ColumnTypeFamily, DefaultValue};

#[derive(Debug)]
pub(crate) struct ColumnDiffer<'a> {
    pub(crate) diffing_options: &'a super::DiffingOptions,
    pub(crate) previous: ColumnRef<'a>,
    pub(crate) next: ColumnRef<'a>,
}

/// On MariaDB, JSON is an alias for LONGTEXT. https://mariadb.com/kb/en/json-data-type/
const MARIADB_ALIASES: &[ColumnTypeFamily] = &[ColumnTypeFamily::String, ColumnTypeFamily::Json];

impl<'a> ColumnDiffer<'a> {
    pub(crate) fn name(&self) -> &'a str {
        debug_assert_eq!(self.previous.name(), self.next.name());

        self.previous.name()
    }

    pub(crate) fn differs_in_something(&self) -> bool {
        self.all_changes().iter().count() > 0
    }

    pub(crate) fn all_changes(&self) -> ColumnChanges {
        let renaming = if self.previous.name() != self.next.name() {
            Some(ColumnChange::Renaming)
        } else {
            None
        };

        let arity = if self.previous.arity() != self.next.arity() {
            Some(ColumnChange::Arity)
        } else {
            None
        };

        let r#type = if self.column_type_changed() {
            Some(ColumnChange::Type)
        } else {
            None
        };

        let default = if !self.defaults_match() {
            Some(ColumnChange::Default)
        } else {
            None
        };

        ColumnChanges {
            changes: [renaming, r#type, arity, default],
        }
    }

    fn column_type_changed(&self) -> bool {
        if self.diffing_options.is_mariadb
            && MARIADB_ALIASES.contains(&self.previous.column_type_family())
            && MARIADB_ALIASES.contains(&self.next.column_type_family())
        {
            return false;
        }

        self.previous.column_type_family() != self.next.column_type_family()
    }

    /// There are workarounds to cope with current migration and introspection limitations.
    ///
    /// - Since the values we set and introspect for timestamps are stringly typed, matching exactly the default value strings does not work on any database. Therefore we consider all datetime defaults as the same.
    ///
    /// - Postgres autoincrement fields get inferred with a default, which we want to ignore.
    ///
    /// - We bail on a number of cases that are too complex to deal with right now or underspecified.
    fn defaults_match(&self) -> bool {
        if self.previous.auto_increment() {
            return true;
        }

        // JSON defaults on MySQL should be ignored.
        if self.diffing_options.sql_family().is_mysql()
            && (self.previous.column_type_family().is_json() || self.next.column_type_family().is_json())
        {
            return true;
        }

        match (&self.previous.default(), &self.next.default()) {
            // Avoid naive string comparisons for JSON defaults.
            (
                Some(DefaultValue::VALUE(PrismaValue::Json(prev_json))),
                Some(DefaultValue::VALUE(PrismaValue::Json(next_json))),
            )
            | (
                Some(DefaultValue::VALUE(PrismaValue::String(prev_json))),
                Some(DefaultValue::VALUE(PrismaValue::Json(next_json))),
            )
            | (
                Some(DefaultValue::VALUE(PrismaValue::Json(prev_json))),
                Some(DefaultValue::VALUE(PrismaValue::String(next_json))),
            ) => json_defaults_match(prev_json, next_json),

            (Some(DefaultValue::VALUE(prev)), Some(DefaultValue::VALUE(next))) => prev == next,
            (Some(DefaultValue::VALUE(_)), Some(DefaultValue::SEQUENCE(_))) => true,
            (Some(DefaultValue::VALUE(_)), Some(DefaultValue::NOW)) => false,
            (Some(DefaultValue::VALUE(_)), None) => false,

            (Some(DefaultValue::NOW), Some(DefaultValue::NOW)) => true,
            (Some(DefaultValue::NOW), Some(DefaultValue::SEQUENCE(_))) => true,
            (Some(DefaultValue::NOW), None) => false,
            (Some(DefaultValue::NOW), Some(DefaultValue::VALUE(_))) => false,

            (Some(DefaultValue::DBGENERATED(_)), Some(DefaultValue::SEQUENCE(_))) => true,
            (Some(DefaultValue::DBGENERATED(_)), Some(DefaultValue::VALUE(_))) => false,
            (Some(DefaultValue::DBGENERATED(_)), Some(DefaultValue::NOW)) => false,
            (Some(DefaultValue::DBGENERATED(_)), None) => false,

            (Some(DefaultValue::SEQUENCE(_)), Some(DefaultValue::SEQUENCE(_))) => true,
            (Some(DefaultValue::SEQUENCE(_)), None) => false,
            (Some(DefaultValue::SEQUENCE(_)), Some(DefaultValue::VALUE(_))) => false,
            (Some(DefaultValue::SEQUENCE(_)), Some(DefaultValue::NOW)) => false,

            (None, None) => true,
            (None, Some(DefaultValue::SEQUENCE(_))) => true,
            (None, Some(DefaultValue::VALUE(_))) => false,
            (None, Some(DefaultValue::NOW)) => false,

            // We can never migrate to @dbgenerated
            (_, Some(DefaultValue::DBGENERATED(_))) => true,
        }
    }
}

fn json_defaults_match(previous: &str, next: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(previous)
        .and_then(|previous| serde_json::from_str::<serde_json::Value>(next).map(|next| (previous, next)))
        .map(|(previous, next)| previous == next)
        .unwrap_or(true)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ColumnChange {
    Renaming,
    Arity,
    Default,
    Type,
}

#[derive(Debug, Clone)]
pub(crate) struct ColumnChanges {
    changes: [Option<ColumnChange>; 4],
}

impl ColumnChanges {
    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = ColumnChange> + 'a {
        self.changes.iter().filter_map(|c| c.as_ref().cloned())
    }

    pub(crate) fn type_changed(&self) -> bool {
        self.changes.iter().any(|c| c.as_ref() == Some(&ColumnChange::Type))
    }

    pub(crate) fn arity_changed(&self) -> bool {
        self.changes.iter().any(|c| c.as_ref() == Some(&ColumnChange::Arity))
    }

    pub(crate) fn only_default_changed(&self) -> bool {
        matches!(self.changes, [None, None, None, Some(ColumnChange::Default)])
    }

    pub(crate) fn column_was_renamed(&self) -> bool {
        matches!(self.changes, [Some(ColumnChange::Renaming), _, _, _])
    }
}
