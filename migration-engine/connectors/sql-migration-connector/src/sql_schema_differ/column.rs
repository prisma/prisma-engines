use crate::{flavour::SqlFlavour, pair::Pair};
use enumflags2::BitFlags;
use prisma_value::PrismaValue;
use sql_schema_describer::{walkers::ColumnWalker, DefaultKind};

#[derive(Debug)]
pub(crate) struct ColumnDiffer<'a> {
    pub(crate) flavour: &'a dyn SqlFlavour,
    pub(crate) previous: ColumnWalker<'a>,
    pub(crate) next: ColumnWalker<'a>,
}

impl<'a> ColumnDiffer<'a> {
    pub(crate) fn all_changes(&self) -> (ColumnChanges, Option<ColumnTypeChange>) {
        let mut changes = BitFlags::empty();
        let column_type_change = self.flavour.column_type_change(self);

        if self.previous.name() != self.next.name() {
            changes |= ColumnChange::Renaming;
        };

        if self.arity_changed() {
            changes |= ColumnChange::Arity
        };

        if column_type_change.is_some() {
            changes |= ColumnChange::TypeChanged;
        };

        if !self.defaults_match() {
            changes |= ColumnChange::Default;
        };

        if self.previous.is_autoincrement() != self.next.is_autoincrement() {
            changes |= ColumnChange::Sequence;
        };

        (ColumnChanges { changes }, column_type_change)
    }

    pub(crate) fn arity_changed(&self) -> bool {
        self.previous.arity() != self.next.arity()
    }

    pub(crate) fn as_pair(&self) -> Pair<&ColumnWalker<'a>> {
        Pair::new(&self.previous, &self.next)
    }

    pub(crate) fn autoincrement_changed(&self) -> bool {
        self.previous.is_autoincrement() != self.next.is_autoincrement()
    }

    /// There are workarounds to cope with current migration and introspection limitations.
    ///
    /// - We bail on a number of cases that are too complex to deal with right now or underspecified.
    fn defaults_match(&self) -> bool {
        // JSON defaults on MySQL should be ignored.
        if self.flavour.should_ignore_json_defaults()
            && (self.previous.column_type_family().is_json() || self.next.column_type_family().is_json())
        {
            return true;
        }

        let defaults = (
            &self.previous.default().as_ref().map(|d| d.kind()),
            &self.next.default().as_ref().map(|d| d.kind()),
        );

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
            ) => json_defaults_match(prev_json, next_json),

            (Some(DefaultKind::Value(prev)), Some(DefaultKind::Value(next))) => prev == next,
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

            (None, None) => true,
            (None, Some(DefaultKind::Value(_))) => false,
            (None, Some(DefaultKind::Now)) => false,

            // We now do migrate to @dbgenerated
            (Some(DefaultKind::DbGenerated(prev)), Some(DefaultKind::DbGenerated(next))) => {
                prev.to_lowercase() == next.to_lowercase()
            }
            (_, Some(DefaultKind::DbGenerated(_))) => false,
            // Sequence migrations are handled separately.
            (_, Some(DefaultKind::Sequence(_))) => true,
        }
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
    Renaming,
    Arity,
    Default,
    TypeChanged,
    Sequence,
}

// This should be pub(crate), but SqlMigration is exported, so it has to be
// public at the moment.
#[derive(Debug, Clone, PartialEq, Default, Eq)]
pub struct ColumnChanges {
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

    #[allow(clippy::needless_lifetimes)] // clippy is wrong here
    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = ColumnChange> + 'a {
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
        self.changes == BitFlags::from(ColumnChange::Default)
    }

    pub(crate) fn only_type_changed(&self) -> bool {
        self.changes == BitFlags::from(ColumnChange::TypeChanged)
    }

    pub(crate) fn column_was_renamed(&self) -> bool {
        self.changes.contains(ColumnChange::Renaming)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnTypeChange {
    SafeCast,
    RiskyCast,
    NotCastable,
}
