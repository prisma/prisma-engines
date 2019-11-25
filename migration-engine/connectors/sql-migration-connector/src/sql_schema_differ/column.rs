use sql_schema_describer::Column;

pub(crate) struct ColumnDiffer<'a> {
    pub(crate) previous: &'a Column,
    pub(crate) next: &'a Column,
}

impl<'a> ColumnDiffer<'a> {
    pub(crate) fn differs_in_something(&self) -> bool {
        self.previous.name != self.next.name
            // TODO: compare the whole type
            // || self.previous.tpe != self.next.tpe
            || self.previous.tpe.family != self.next.tpe.family
            || self.previous.arity != self.next.arity
            || self.previous.default != self.next.default
    }
}
