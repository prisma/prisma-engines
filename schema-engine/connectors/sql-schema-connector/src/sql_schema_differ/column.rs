use crate::{flavour::SqlFlavour, migration_pair::MigrationPair};
use enumflags2::BitFlags;

use sql_schema_describer::{walkers::TableColumnWalker, DefaultKind, PrismaValue};

pub(crate) fn all_changes(cols: MigrationPair<TableColumnWalker<'_>>, flavour: &dyn SqlFlavour) -> ColumnChanges {
    let mut changes = BitFlags::empty();
    let type_change = flavour.column_type_change(cols);

    if cols.previous.arity() != cols.next.arity() {
        changes |= ColumnChange::Arity
    };

    if type_change.is_some() {
        changes |= ColumnChange::TypeChanged;
    };

    if !defaults_match(cols, flavour) {
        changes |= ColumnChange::Default;
    };

    if flavour.column_autoincrement_changed(cols) {
        changes |= ColumnChange::Autoincrement;
    }

    ColumnChanges { type_change, changes }
}

/// There are workarounds to cope with current migration and introspection limitations.
///
/// - We bail on a number of cases that are too complex to deal with right now or underspecified.
fn defaults_match(cols: MigrationPair<TableColumnWalker<'_>>, flavour: &dyn SqlFlavour) -> bool {
    // JSON defaults on MySQL should be ignored.
    if flavour.should_ignore_json_defaults()
        && (cols.previous.column_type_family().is_json() || cols.next.column_type_family().is_json())
    {
        return true;
    }

    let prev = cols.previous.default();
    let next = cols.next.default();

    let defaults = (prev.map(|d| d.kind()), next.map(|d| d.kind()));

    let names_match = {
        let prev_constraint = prev.and_then(|v| v.constraint_name());
        let next_constraint = next.and_then(|v| v.constraint_name());

        prev_constraint == next_constraint
    };

    match defaults {
        (Some(DefaultKind::DbGenerated(_)), Some(DefaultKind::Value(PrismaValue::List(_))))
        | (Some(DefaultKind::Value(PrismaValue::List(_))), Some(DefaultKind::DbGenerated(_)))
            if cols.previous.column_type_family().is_datetime() || cols.next.column_type_family().is_datetime() =>
        {
            true
        }

        (Some(DefaultKind::Value(PrismaValue::List(prev))), Some(DefaultKind::Value(PrismaValue::List(next)))) => {
            list_defaults_match(prev, next, flavour)
        }

        // Avoid naive string comparisons for JSON defaults.
        (
            Some(DefaultKind::Value(PrismaValue::Json(prev_json))),
            Some(DefaultKind::Value(PrismaValue::Json(next_json))),
        )
        | (
            Some(DefaultKind::Value(PrismaValue::String(prev_json))),
            Some(DefaultKind::Value(PrismaValue::Json(next_json))),
        )
        | (
            Some(DefaultKind::Value(PrismaValue::Json(prev_json))),
            Some(DefaultKind::Value(PrismaValue::String(next_json))),
        ) => json_defaults_match(prev_json, next_json) && names_match,

        // Avoid naive string comparisons for datetime defaults.
        (Some(DefaultKind::Value(PrismaValue::DateTime(_))), Some(_))
        | (Some(_), Some(DefaultKind::Value(PrismaValue::DateTime(_)))) => true, // can't diff these in at present

        (Some(DefaultKind::Value(PrismaValue::Int(i))), Some(DefaultKind::Value(PrismaValue::BigInt(j))))
        | (Some(DefaultKind::Value(PrismaValue::BigInt(i))), Some(DefaultKind::Value(PrismaValue::Int(j)))) => i == j,
        (Some(DefaultKind::Value(prev)), Some(DefaultKind::Value(next))) => (prev == next) && names_match,
        (Some(DefaultKind::Value(_)), Some(DefaultKind::Now)) => false,
        (Some(DefaultKind::Value(_)), None) => false,

        (Some(DefaultKind::Now), Some(DefaultKind::Now)) => true,
        (Some(DefaultKind::Now), None) => false,
        (Some(DefaultKind::Now), Some(DefaultKind::Value(_))) => false,

        (Some(DefaultKind::DbGenerated(_)), Some(DefaultKind::Value(_))) => false,
        (Some(DefaultKind::DbGenerated(_)), Some(DefaultKind::Now)) => false,
        (Some(DefaultKind::DbGenerated(_)), None) => false,
        (_, Some(DefaultKind::DbGenerated(None))) => true,

        (Some(DefaultKind::Sequence(_)), None) => true, // sequences are dropped separately
        (Some(DefaultKind::Sequence(_)), Some(DefaultKind::Value(_))) => false,
        (Some(DefaultKind::Sequence(_)), Some(DefaultKind::Now)) => false,

        (Some(DefaultKind::UniqueRowid), Some(DefaultKind::UniqueRowid)) => true,
        (Some(DefaultKind::UniqueRowid), _) | (_, Some(DefaultKind::UniqueRowid)) => false,

        (None, None) => true,
        (None, Some(DefaultKind::Value(_))) => false,
        (None, Some(DefaultKind::Now)) => false,

        (Some(DefaultKind::DbGenerated(Some(prev))), Some(DefaultKind::DbGenerated(Some(next)))) => {
            (prev.eq_ignore_ascii_case(next)) && names_match
        }
        (_, Some(DefaultKind::DbGenerated(_))) => false,
        (_, Some(DefaultKind::Sequence(_))) => true,
    }
}

fn json_defaults_match(previous: &str, next: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(previous)
        .and_then(|previous| serde_json::from_str::<serde_json::Value>(next).map(|next| (previous, next)))
        .map(|(previous, next)| previous == next)
        .unwrap_or(true)
}

fn list_defaults_match(prev: &[PrismaValue], next: &[PrismaValue], flavour: &dyn SqlFlavour) -> bool {
    if prev.len() != next.len() {
        return false;
    }

    prev.iter()
        .zip(next.iter())
        .all(|(prev_value, next_value)| match (prev_value, next_value) {
            (PrismaValue::String(string_val), PrismaValue::Json(json_val))
            | (PrismaValue::Json(json_val), PrismaValue::String(string_val)) => {
                json_defaults_match(string_val, json_val)
            }

            (PrismaValue::DateTime(_), _) | (_, PrismaValue::DateTime(_)) => true,

            (PrismaValue::Enum(enum_val), PrismaValue::Bytes(bytes_val))
            | (PrismaValue::Bytes(bytes_val), PrismaValue::Enum(enum_val)) => {
                flavour.string_matches_bytes(enum_val, bytes_val)
            }

            _ => prev_value == next_value,
        })
}

#[enumflags2::bitflags]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum ColumnChange {
    Arity,
    Default,
    TypeChanged,
    Autoincrement,
}

// This should be pub(crate), but SqlMigration is exported, so it has to be
// public at the moment.
#[derive(Debug, Clone, PartialEq, Default, Eq, Copy)]
pub(crate) struct ColumnChanges {
    pub(crate) type_change: Option<ColumnTypeChange>,
    changes: BitFlags<ColumnChange>,
}

impl PartialOrd for ColumnChanges {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ColumnChanges {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.changes.bits().cmp(&other.changes.bits())
    }
}

impl ColumnChanges {
    pub(crate) fn differs_in_something(&self) -> bool {
        !self.changes.is_empty()
    }

    pub(crate) fn autoincrement_changed(&self) -> bool {
        self.changes.contains(ColumnChange::Autoincrement)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = ColumnChange> + '_ {
        self.changes.iter()
    }

    pub(crate) fn type_changed(&self) -> bool {
        self.changes.contains(ColumnChange::TypeChanged)
    }

    pub(crate) fn arity_changed(&self) -> bool {
        self.changes.contains(ColumnChange::Arity)
    }

    pub(crate) fn default_changed(&self) -> bool {
        self.changes.contains(ColumnChange::Default)
    }

    pub(crate) fn only_default_changed(&self) -> bool {
        self.changes == ColumnChange::Default
    }

    pub(crate) fn only_type_changed(&self) -> bool {
        self.changes == ColumnChange::TypeChanged
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ColumnTypeChange {
    SafeCast,
    RiskyCast,
    NotCastable,
}
