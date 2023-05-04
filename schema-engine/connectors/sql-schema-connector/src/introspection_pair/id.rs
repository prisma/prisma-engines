use psl::{datamodel_connector::constraint_names::ConstraintNames, parser_database::walkers};
use sql_schema_describer as sql;

use super::{IndexFieldPair, IntrospectionPair};

/// Pairing PSL id to database primary keys. Both values are
/// optional, due to in some cases we plainly just copy
/// the PSL attribute to the rendered data model.
///
/// This happens with views, where we need at least one unique
/// field in the view definition, but the database does not
/// hold constraints on views.
pub(crate) type IdPair<'a> = IntrospectionPair<'a, Option<walkers::PrimaryKeyWalker<'a>>, Option<sql::IndexWalker<'a>>>;

impl<'a> IdPair<'a> {
    /// The user-facing name of the identifier, defined solely in the
    /// PSL.
    pub(crate) fn name(self) -> Option<&'a str> {
        self.previous.and_then(|id| id.name())
    }

    /// The constraint name in the database, if non-default.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        match self.next {
            Some(next) => {
                let default = ConstraintNames::primary_key_name(next.table().name(), self.context.active_connector());
                let name = next.name();

                (!name.is_empty() && name != default).then_some(name)
            }
            None => self.previous.and_then(|prev| prev.mapped_name()),
        }
    }

    /// SQL Server specific clustering setting. A value is returned if
    /// non-default.
    #[cfg(feature = "mssql")]
    pub(crate) fn clustered(self) -> Option<bool> {
        if !self.context.sql_family.is_mssql() {
            return None;
        }

        let clustered = match self.next {
            Some(next) => {
                let ext: &sql::mssql::MssqlSchemaExt = self.context.sql_schema.downcast_connector_data();
                ext.index_is_clustered(next.id)
            }
            None => self.previous.and_then(|prev| prev.clustered()).unwrap_or(true),
        };

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
    pub(crate) fn fields(self) -> Box<dyn ExactSizeIterator<Item = IndexFieldPair<'a>> + 'a> {
        match self.next {
            Some(next) => Box::new(next.columns().enumerate().map(move |(i, c)| {
                let previous = self.previous.and_then(|prev| prev.fields().nth(i));
                IntrospectionPair::new(self.context, previous, Some(c))
            })),
            None => Box::new(
                self.previous
                    .unwrap()
                    .fields()
                    .map(move |previous| IntrospectionPair::new(self.context, Some(previous), None)),
            ),
        }
    }
}
