use psl::{datamodel_connector::constraint_names::ConstraintNames, parser_database::walkers};
use sql::mssql::MssqlSchemaExt;
use sql_schema_describer as sql;

use super::{IndexFieldPair, Pair};

pub(crate) type IdPair<'a> = Pair<'a, walkers::PrimaryKeyWalker<'a>, sql::IndexWalker<'a>>;

impl<'a> IdPair<'a> {
    /// The user-facing name of the identifier, defined solely in the
    /// PSL.
    pub(crate) fn name(self) -> Option<&'a str> {
        self.previous.and_then(|id| id.name())
    }

    /// The constraint name in the database, if non-default.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        let name = self.next.name();
        (!name.is_empty() && name != self.default_constraint_name()).then_some(name)
    }

    /// SQL Server specific clustering setting. A value is returned if
    /// non-default.
    pub(crate) fn clustered(self) -> Option<bool> {
        if !self.context.sql_family.is_mssql() {
            return None;
        }

        let ext: &MssqlSchemaExt = self.context.schema.downcast_connector_data();
        let clustered = ext.index_is_clustered(self.next.id);

        if clustered {
            return None;
        }

        Some(clustered)
    }

    /// True if the `@id` attribute is in a field, not in the model as
    /// `@@id`.
    pub(crate) fn defined_in_a_field(self) -> bool {
        self.fields().len() == 1
    }

    /// If defined in a single field, returns the given field.
    pub(crate) fn field(self) -> Option<IndexFieldPair<'a>> {
        self.defined_in_a_field().then(|| self.fields().next().unwrap())
    }

    /// The fields the primary key is consisting of.
    pub(crate) fn fields(self) -> impl ExactSizeIterator<Item = IndexFieldPair<'a>> {
        self.next.columns().enumerate().map(move |(i, c)| {
            let previous = self.previous.and_then(|prev| prev.fields().nth(i));
            Pair::new(self.context, previous, c)
        })
    }

    fn default_constraint_name(self) -> String {
        ConstraintNames::primary_key_name(self.next.table().name(), self.context.active_connector())
    }
}
