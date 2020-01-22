use sql_schema_describer::{Column, ColumnTypeFamily};

#[derive(Debug)]
pub(crate) struct ColumnDiffer<'a> {
    pub(crate) previous: &'a Column,
    pub(crate) next: &'a Column,
}

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

        let r#type = if self.previous.tpe.family != self.next.tpe.family {
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

    /// There are workarounds to cope with current migration and introspection limitations.
    ///
    /// - Since the values we set and introspect for timestamps are stringly typed, matching exactly the default value strings does not work on any database. Therefore we consider all datetime defaults as the same.
    ///
    /// - Postgres autoincrement fields get inferred with a default, which we want to ignore.
    ///
    /// - We bail on a number of cases that are too complex to deal with right now or underspecified, like strings containing escaped characters.
    fn defaults_match(&self) -> bool {
        if self.previous.auto_increment {
            return true;
        }

        let previous_value: Option<&str> = self.previous.default.as_ref().map(String::as_str);
        let next_value: Option<&str> = self.next.default.as_ref().map(String::as_str);

        match self.previous.tpe.family {
            ColumnTypeFamily::String => string_defaults_match(previous_value, next_value),
            ColumnTypeFamily::Float => float_default(previous_value) == float_default(next_value),
            ColumnTypeFamily::Int => int_default(previous_value) == int_default(next_value),
            ColumnTypeFamily::Boolean => bool_default(previous_value) == bool_default(next_value),
            _ => true,
        }
    }
}

fn float_default(s: Option<&str>) -> Option<f64> {
    s.and_then(|s| s.parse().ok())
}

fn int_default(s: Option<&str>) -> Option<i128> {
    s.and_then(|s| s.parse().ok())
}

fn bool_default(s: Option<&str>) -> Option<bool> {
    s.and_then(|s| match s {
        "true" | "TRUE" | "True" | "t" | "1" => Some(true),
        "false" | "FALSE" | "False" | "f" | "0" => Some(false),
        _ => None,
    })
}

fn string_defaults_match(previous: Option<&str>, next: Option<&str>) -> bool {
    match (previous, next) {
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => true,
        (Some(previous), Some(next)) => {
            if string_contains_tricky_character(previous) || string_contains_tricky_character(next) {
                return true;
            }

            previous == next
        }
    }
}

fn string_contains_tricky_character(s: &str) -> bool {
    s.contains('\\') || s.contains("'") || s.contains("\"")
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_schema_describer::{ColumnArity, ColumnType, ColumnTypeFamily};

    #[test]
    fn quoted_string_defaults_match() {
        let col_a = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::String, ColumnArity::Required),
            default: Some("abc".to_owned()),
            auto_increment: false,
        };

        let col_b = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::String, ColumnArity::Required),
            default: Some(r##""abc""##.to_owned()),
            auto_increment: false,
        };

        let col_c = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::String, ColumnArity::Required),
            default: Some(r##"'abc'"##.to_owned()),
            auto_increment: false,
        };

        assert!(ColumnDiffer {
            previous: &col_a,
            next: &col_b
        }
        .defaults_match());

        assert!(ColumnDiffer {
            previous: &col_a,
            next: &col_c
        }
        .defaults_match());

        assert!(ColumnDiffer {
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
            default: Some("2019-09-01T08:00:00Z".to_owned()),
            auto_increment: false,
        };

        let col_b = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::DateTime, ColumnArity::Required),
            default: Some("2019-09-01 18:00:00 UTC".to_owned()),
            auto_increment: false,
        };

        assert!(ColumnDiffer {
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
            default: Some("0.33".to_owned()),
            auto_increment: false,
        };

        let col_b = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::Float, ColumnArity::Required),
            default: Some("0.33000".to_owned()),
            auto_increment: false,
        };

        assert!(ColumnDiffer {
            previous: &col_a,
            next: &col_b,
        }
        .defaults_match());

        let col_c = Column {
            name: "A".to_owned(),
            tpe: ColumnType::pure(ColumnTypeFamily::Float, ColumnArity::Required),
            default: Some("0.34".to_owned()),
            auto_increment: false,
        };

        assert!(!ColumnDiffer {
            previous: &col_a,
            next: &col_c,
        }
        .defaults_match());
    }
}
