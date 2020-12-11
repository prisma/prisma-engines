use crate::flavour::SqlFlavour;
use enumflags2::BitFlags;
use prisma_value::PrismaValue;
use sql_schema_describer::{walkers::ColumnWalker, ColumnTypeFamily, DefaultKind};

#[derive(Debug)]
pub(crate) struct ColumnDiffer<'a> {
    pub(crate) flavour: &'a dyn SqlFlavour,
    pub(crate) previous: ColumnWalker<'a>,
    pub(crate) next: ColumnWalker<'a>,
}

impl<'a> ColumnDiffer<'a> {
    pub(crate) fn all_changes(&self) -> (ColumnChanges, Option<ColumnTypeChange>) {
        let mut changes = BitFlags::empty();
        let column_type_change = self.column_type_change();

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

    pub(crate) fn autoincrement_changed(&self) -> bool {
        self.previous.is_autoincrement() != self.next.is_autoincrement()
    }

    fn column_type_change(&self) -> Option<ColumnTypeChange> {
        match (self.previous.column_type_family(), self.next.column_type_family()) {
            (_, _) if self.arity_changed() => self.flavour.column_type_change(self),
            (ColumnTypeFamily::Decimal, ColumnTypeFamily::Decimal) => None,
            (ColumnTypeFamily::Decimal, ColumnTypeFamily::Float) => None,
            (ColumnTypeFamily::Float, ColumnTypeFamily::Decimal) => None,
            (ColumnTypeFamily::Float, ColumnTypeFamily::Float) => None,
            (ColumnTypeFamily::String, ColumnTypeFamily::Uuid) => None,
            (ColumnTypeFamily::Uuid, ColumnTypeFamily::String) => None,
            (_, _) => self.flavour.column_type_change(self),
        }
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
                Some(DefaultKind::VALUE(PrismaValue::Json(prev_json))),
                Some(DefaultKind::VALUE(PrismaValue::Json(next_json))),
            )
            | (
                Some(DefaultKind::VALUE(PrismaValue::String(prev_json))),
                Some(DefaultKind::VALUE(PrismaValue::Json(next_json))),
            )
            | (
                Some(DefaultKind::VALUE(PrismaValue::Json(prev_json))),
                Some(DefaultKind::VALUE(PrismaValue::String(next_json))),
            ) => json_defaults_match(prev_json, next_json),

            (Some(DefaultKind::VALUE(prev)), Some(DefaultKind::VALUE(next))) => prev == next,
            (Some(DefaultKind::VALUE(_)), Some(DefaultKind::NOW)) => false,
            (Some(DefaultKind::VALUE(_)), None) => false,

            (Some(DefaultKind::NOW), Some(DefaultKind::NOW)) => true,
            (Some(DefaultKind::NOW), None) => false,
            (Some(DefaultKind::NOW), Some(DefaultKind::VALUE(_))) => false,

            (Some(DefaultKind::DBGENERATED(_)), Some(DefaultKind::VALUE(_))) => false,
            (Some(DefaultKind::DBGENERATED(_)), Some(DefaultKind::NOW)) => false,
            (Some(DefaultKind::DBGENERATED(_)), None) => false,

            (Some(DefaultKind::SEQUENCE(_)), None) => true, // sequences are dropped separately
            (Some(DefaultKind::SEQUENCE(_)), Some(DefaultKind::VALUE(_))) => false,
            (Some(DefaultKind::SEQUENCE(_)), Some(DefaultKind::NOW)) => false,

            (None, None) => true,
            (None, Some(DefaultKind::VALUE(_))) => false,
            (None, Some(DefaultKind::NOW)) => false,

            // We can never migrate to @dbgenerated
            (_, Some(DefaultKind::DBGENERATED(_))) => true,
            // Sequence migrations are handled separately.
            (_, Some(DefaultKind::SEQUENCE(_))) => true,
        }
    }
}

fn json_defaults_match(previous: &str, next: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(previous)
        .and_then(|previous| serde_json::from_str::<serde_json::Value>(next).map(|next| (previous, next)))
        .map(|(previous, next)| previous == next)
        .unwrap_or(true)
}

#[derive(BitFlags, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub(crate) enum ColumnChange {
    Renaming = 0b0001,
    Arity = 0b0010,
    Default = 0b0100,
    TypeChanged = 0b1000,
    Sequence = 0b0010000,
}

// This should be pub(crate), but SqlMigration is exported, so it has to be
// public at the moment.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ColumnChanges {
    changes: BitFlags<ColumnChange>,
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
