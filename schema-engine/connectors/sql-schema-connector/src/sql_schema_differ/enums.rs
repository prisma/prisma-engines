use sql_schema_describer::walkers::EnumWalker;

use crate::migration_pair::MigrationPair;

pub(crate) struct EnumDiffer<'a> {
    pub(crate) enums: MigrationPair<EnumWalker<'a>>,
}

impl<'a> EnumDiffer<'a> {
    pub(crate) fn created_values<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        self.enums.next.values().filter(move |next_value| {
            !self
                .enums
                .previous
                .values()
                .any(|previous_value| values_match(previous_value, next_value))
        })
    }

    pub(crate) fn dropped_values<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        self.enums.previous.values().filter(move |previous_value| {
            !self
                .enums
                .next
                .values()
                .any(|next_value| values_match(previous_value, next_value))
        })
    }
}

fn values_match(previous: &str, next: &str) -> bool {
    previous == next
}
