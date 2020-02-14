use sql_schema_describer::Enum;

pub(crate) struct EnumDiffer<'a> {
    pub(crate) previous: &'a Enum,
    pub(crate) next: &'a Enum,
}

impl<'a> EnumDiffer<'a> {
    pub(crate) fn created_values<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        self.next
            .values
            .iter()
            .filter(move |next_value| {
                !self
                    .previous
                    .values
                    .iter()
                    .any(|previous_value| values_match(previous_value, next_value))
            })
            .map(String::as_str)
    }

    pub(crate) fn dropped_values<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        self.previous
            .values
            .iter()
            .filter(move |previous_value| {
                !self
                    .next
                    .values
                    .iter()
                    .any(|next_value| values_match(previous_value, next_value))
            })
            .map(String::as_str)
    }
}

fn values_match(previous: &str, next: &str) -> bool {
    previous == next
}
