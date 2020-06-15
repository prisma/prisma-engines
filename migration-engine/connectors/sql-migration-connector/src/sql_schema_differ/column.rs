use sql_schema_describer::{Column, ColumnTypeFamily, DefaultValue};

#[derive(Debug)]
pub(crate) struct ColumnDiffer<'a> {
    pub(crate) diffing_options: &'a super::DiffingOptions,
    pub(crate) previous: &'a Column,
    pub(crate) next: &'a Column,
}

/// On MariaDB, JSON is an alias for LONGTEXT. https://mariadb.com/kb/en/json-data-type/
const MARIADB_ALIASES: &[ColumnTypeFamily] = &[ColumnTypeFamily::String, ColumnTypeFamily::Json];

impl<'a> ColumnDiffer<'a> {
    pub(crate) fn name(&self) -> &'a str {
        debug_assert_eq!(self.previous.name, self.next.name);

        self.previous.name.as_str()
    }

    pub(crate) fn differs_in_something(&self) -> bool {
        self.all_changes().iter().count() > 0
    }

    pub(crate) fn all_changes(&self) -> ColumnChanges {
        let renaming = if self.previous.name != self.next.name {
            Some(ColumnChange::Renaming)
        } else {
            None
        };

        let arity = if self.previous.tpe.arity != self.next.tpe.arity {
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
            && MARIADB_ALIASES.contains(&self.previous.tpe.family)
            && MARIADB_ALIASES.contains(&self.next.tpe.family)
        {
            return false;
        }

        self.previous.tpe.family != self.next.tpe.family
    }

    /// There are workarounds to cope with current migration and introspection limitations.
    ///
    /// - Since the values we set and introspect for timestamps are stringly typed, matching exactly the default value strings does not work on any database. Therefore we consider all datetime defaults as the same.
    ///
    /// - Postgres autoincrement fields get inferred with a default, which we want to ignore.
    ///
    /// - We bail on a number of cases that are too complex to deal with right now or underspecified.
    fn defaults_match(&self) -> bool {
        if self.previous.auto_increment {
            return true;
        }

        match (&self.previous.default, &self.next.default) {
            (Some(DefaultValue::VALUE(prev)), Some(DefaultValue::VALUE(next))) => prev == next,
            (Some(DefaultValue::VALUE(_)), Some(DefaultValue::DBGENERATED(_))) => true,
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
        self.changes.iter().filter_map(|c| c.as_ref().map(|c| c.clone()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use prisma_value::PrismaValue;
    use sql_schema_describer::{ColumnArity, ColumnType, ColumnTypeFamily, DefaultValue};

    #[test]
    #[ignore] // these values should already be cleaned up during introspection / datamodel validation
    fn quoted_string_defaults_match() {
        let col_a = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::String, ColumnArity::Required),
            default: Some(DefaultValue::VALUE(PrismaValue::String("abc".to_owned()))),
            auto_increment: false,
        };

        let col_b = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::String, ColumnArity::Required),
            default: Some(DefaultValue::VALUE(PrismaValue::String(r##""abc""##.to_owned()))),
            auto_increment: false,
        };

        let col_c = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::String, ColumnArity::Required),
            default: Some(DefaultValue::VALUE(PrismaValue::String(r##"'abc'"##.to_owned()))),
            auto_increment: false,
        };

        assert!(ColumnDiffer {
            diffing_options: &Default::default(),
            previous: &col_a,
            next: &col_b
        }
        .defaults_match());

        assert!(ColumnDiffer {
            diffing_options: &Default::default(),
            previous: &col_a,
            next: &col_c
        }
        .defaults_match());

        assert!(ColumnDiffer {
            diffing_options: &Default::default(),
            previous: &col_c,
            next: &col_b
        }
        .defaults_match());
    }

    #[test]
    fn datetime_defaults_match() {
        let col_a = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::DateTime, ColumnArity::Required),
            default: Some(DefaultValue::VALUE(PrismaValue::new_datetime("2019-09-01T08:00:00Z"))),
            auto_increment: false,
        };

        let col_b = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::DateTime, ColumnArity::Required),
            default: Some(DefaultValue::VALUE(PrismaValue::new_datetime(
                "2019-09-01 08:00:00 UTC",
            ))),
            auto_increment: false,
        };

        assert!(ColumnDiffer {
            diffing_options: &Default::default(),
            previous: &col_a,
            next: &col_b,
        }
        .defaults_match());
    }

    #[test]
    fn float_defaults_match() {
        let col_a = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::Float, ColumnArity::Required),
            default: Some(DefaultValue::VALUE(PrismaValue::new_float(0.33))),
            auto_increment: false,
        };

        let col_b = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::Float, ColumnArity::Required),
            default: Some(DefaultValue::VALUE(PrismaValue::new_float(0.3300))),
            auto_increment: false,
        };

        assert!(ColumnDiffer {
            diffing_options: &Default::default(),
            previous: &col_a,
            next: &col_b,
        }
        .defaults_match());

        let col_c = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::Float, ColumnArity::Required),
            default: Some(DefaultValue::VALUE(PrismaValue::new_float(0.34))),
            auto_increment: false,
        };

        assert!(!ColumnDiffer {
            diffing_options: &Default::default(),
            previous: &col_a,
            next: &col_c,
        }
        .defaults_match());
    }
}
