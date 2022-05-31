use crate::{flavour::SqlFlavour, pair::Pair};
use datamodel::dml::PrismaValue;
use enumflags2::BitFlags;
use sql_schema_describer::{walkers::ColumnWalker, DefaultKind};

pub(crate) fn all_changes(cols: Pair<ColumnWalker<'_>>, flavour: &dyn SqlFlavour) -> ColumnChanges {
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
fn defaults_match(cols: Pair<ColumnWalker<'_>>, flavour: &dyn SqlFlavour) -> bool {
    // JSON defaults on MySQL should be ignored.
    if flavour.should_ignore_json_defaults()
        && (cols.previous.column_type_family().is_json() || cols.next.column_type_family().is_json())
    {
        return true;
    }

    if cols.map(|c| c.arity().is_list()).into_tuple() == (true, true) {
        return true; // TODO: diff scalar list defaults
    }

    let prev = cols.previous().default();
    let next = cols.next().default();

    let defaults = (&prev.as_ref().map(|d| d.kind()), &next.as_ref().map(|d| d.kind()));

    let names_match = {
        let prev_constraint = prev.and_then(|v| v.constraint_name());
        let next_constraint = next.and_then(|v| v.constraint_name());

        prev_constraint == next_constraint
    };

    match defaults {
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

        (Some(DefaultKind::Value(prev)), Some(DefaultKind::Value(next))) => (prev == next) && names_match,
        (Some(DefaultKind::Value(_)), Some(DefaultKind::Now)) => false,
        (Some(DefaultKind::Value(_)), None) => false,

        (Some(DefaultKind::Now), Some(DefaultKind::Now)) => true,
        (Some(DefaultKind::Now), None) => false,
        (Some(DefaultKind::Now), Some(DefaultKind::Value(_))) => false,

        (Some(DefaultKind::DbGenerated(_)), Some(DefaultKind::Value(_))) => false,
        (Some(DefaultKind::DbGenerated(_)), Some(DefaultKind::Now)) => false,
        (Some(DefaultKind::DbGenerated(_)), None) => false,

        (Some(DefaultKind::Sequence(_)), None) => true, // sequences are dropped separately
        (Some(DefaultKind::Sequence(_)), Some(DefaultKind::Value(_))) => false,
        (Some(DefaultKind::Sequence(_)), Some(DefaultKind::Now)) => false,

        (Some(DefaultKind::UniqueRowid), Some(DefaultKind::UniqueRowid)) => true,
        (Some(DefaultKind::UniqueRowid), _) | (_, Some(DefaultKind::UniqueRowid)) => false,

        (None, None) => true,
        (None, Some(DefaultKind::Value(_))) => false,
        (None, Some(DefaultKind::Now)) => false,

        (Some(DefaultKind::DbGenerated(prev)), Some(DefaultKind::DbGenerated(next))) => {
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
        self.changes.bits().partial_cmp(&other.changes.bits())
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
