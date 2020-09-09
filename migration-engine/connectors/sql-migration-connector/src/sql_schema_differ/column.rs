use crate::{database_info::DatabaseInfo, flavour::SqlFlavour};
use enumflags2::BitFlags;
use prisma_value::PrismaValue;
use sql_schema_describer::{walkers::ColumnWalker, DefaultValue};

#[derive(Debug)]
pub(crate) struct ColumnDiffer<'a> {
    pub(crate) flavour: &'a dyn SqlFlavour,
    pub(crate) database_info: &'a DatabaseInfo,
    pub(crate) previous: ColumnWalker<'a>,
    pub(crate) next: ColumnWalker<'a>,
}

impl<'a> ColumnDiffer<'a> {
    pub(crate) fn name(&self) -> &'a str {
        debug_assert_eq!(self.previous.name(), self.next.name());

        self.previous.name()
    }

    pub(crate) fn differs_in_something(&self) -> bool {
        self.all_changes().iter().count() > 0
    }

    pub(crate) fn all_changes(&self) -> ColumnChanges {
        let mut changes = BitFlags::empty();

        if self.previous.name() != self.next.name() {
            changes |= ColumnChange::Renaming;
        };

        if self.previous.arity() != self.next.arity() {
            changes |= ColumnChange::Arity
        };

        if self.column_type_changed() {
            changes |= ColumnChange::TypeChanged;
        };

        if !self.defaults_match() {
            changes |= ColumnChange::Default;
        };

        if self.previous.is_autoincrement() != self.next.is_autoincrement() {
            changes |= ColumnChange::Sequence;
        };

        ColumnChanges { changes }
    }

    fn column_type_changed(&self) -> bool {
        self.flavour.column_type_changed(self)
    }

    /// There are workarounds to cope with current migration and introspection limitations.
    ///
    /// - We bail on a number of cases that are too complex to deal with right now or underspecified.
    fn defaults_match(&self) -> bool {
        // JSON defaults on MySQL should be ignored.
        if self.flavour.sql_family().is_mysql()
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
            (Some(DefaultValue::VALUE(_)), Some(DefaultValue::NOW)) => false,
            (Some(DefaultValue::VALUE(_)), None) => false,

            (Some(DefaultValue::NOW), Some(DefaultValue::NOW)) => true,
            (Some(DefaultValue::NOW), None) => false,
            (Some(DefaultValue::NOW), Some(DefaultValue::VALUE(_))) => false,

            (Some(DefaultValue::DBGENERATED(_)), Some(DefaultValue::VALUE(_))) => false,
            (Some(DefaultValue::DBGENERATED(_)), Some(DefaultValue::NOW)) => false,
            (Some(DefaultValue::DBGENERATED(_)), None) => false,

            (Some(DefaultValue::SEQUENCE(_)), None) => true, // sequences are dropped separately
            (Some(DefaultValue::SEQUENCE(_)), Some(DefaultValue::VALUE(_))) => false,
            (Some(DefaultValue::SEQUENCE(_)), Some(DefaultValue::NOW)) => false,

            (None, None) => true,
            (None, Some(DefaultValue::VALUE(_))) => false,
            (None, Some(DefaultValue::NOW)) => false,

            // We can never migrate to @dbgenerated
            (_, Some(DefaultValue::DBGENERATED(_))) => true,
            // Sequence migrations are handled separately.
            (_, Some(DefaultValue::SEQUENCE(_))) => true,
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

#[derive(Debug, Clone)]
pub(crate) struct ColumnChanges {
    changes: BitFlags<ColumnChange>,
}

impl ColumnChanges {
    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = ColumnChange> + 'a {
        self.changes.iter()
    }

    pub(crate) fn type_changed(&self) -> bool {
        self.changes.contains(ColumnChange::TypeChanged)
    }

    pub(crate) fn arity_changed(&self) -> bool {
        self.changes.contains(ColumnChange::Arity)
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
