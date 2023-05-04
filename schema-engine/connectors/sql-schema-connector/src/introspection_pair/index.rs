use psl::{
    datamodel_connector::constraint_names::ConstraintNames,
    parser_database::{walkers, IndexType},
    schema_ast::ast,
    PreviewFeature,
};
use sql_schema_describer as sql;

use super::{IndexFieldPair, IntrospectionPair};

/// Pairing a PSL index to a database index. Both values are
/// optional, due to in some cases we plainly just copy
/// the PSL attribute to the rendered data model.
///
/// This happens with views, where we need at least one unique
/// field in the view definition, but the database does not
/// hold constraints on views.
pub(crate) type IndexPair<'a> = IntrospectionPair<'a, Option<walkers::IndexWalker<'a>>, Option<sql::IndexWalker<'a>>>;

impl<'a> IndexPair<'a> {
    /// The position of the index from the PSL, if existing. Used for
    /// sorting the indexes in the final introspected data model.
    pub(crate) fn previous_position(self) -> Option<ast::AttributeId> {
        self.previous.map(|idx| idx.attribute_id())
    }

    /// The user-facing name of the index, defined solely in the
    /// PSL.
    pub(crate) fn name(self) -> Option<&'a str> {
        self.previous.and_then(|i| i.name())
    }

    /// The constraint name in the database, if non-default.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        match self.next {
            Some(next) => {
                let columns = next.column_names().collect::<Vec<_>>();

                let default = match next.index_type() {
                    sql::IndexType::Unique => ConstraintNames::unique_index_name(
                        next.table().name(),
                        &columns,
                        self.context.active_connector(),
                    ),
                    _ => ConstraintNames::non_unique_index_name(
                        next.table().name(),
                        &columns,
                        self.context.active_connector(),
                    ),
                };

                (next.name() != default).then(|| next.name())
            }
            None => self.previous.and_then(|prev| prev.mapped_name()),
        }
    }

    /// The type of the index.
    pub(crate) fn index_type(self) -> sql::IndexType {
        let preview_features = self.context.config.preview_features();

        match self.next.map(|next| next.index_type()) {
            Some(sql::IndexType::Fulltext) if !preview_features.contains(PreviewFeature::FullTextIndex) => {
                sql::IndexType::Normal
            }
            Some(typ) => typ,
            None => match self.previous.map(|prev| prev.index_type()) {
                Some(IndexType::Unique) => sql::IndexType::Unique,
                Some(IndexType::Fulltext) => sql::IndexType::Fulltext,
                _ => sql::IndexType::Normal,
            },
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
            None => self.previous.and_then(|prev| prev.clustered()).unwrap_or(false),
        };

        if !clustered {
            None
        } else {
            Some(clustered)
        }
    }

    /// A PostgreSQL specific algorithm. Defines the data structure
    /// that defines the index.
    #[cfg(feature = "postgresql")]
    pub(crate) fn algorithm(self) -> Option<&'static str> {
        if !self.context.sql_family().is_postgres() {
            return None;
        }

        match (self.next, self.previous.and_then(|i| i.algorithm())) {
            // Index is defined in a table to the database.
            (Some(next), _) => {
                let data: &sql::postgres::PostgresSchemaExt = self.context.sql_schema.downcast_connector_data();

                match data.index_algorithm(next.id) {
                    sql::postgres::SqlIndexAlgorithm::BTree => None,
                    sql::postgres::SqlIndexAlgorithm::Hash => Some("Hash"),
                    sql::postgres::SqlIndexAlgorithm::Gist => Some("Gist"),
                    sql::postgres::SqlIndexAlgorithm::Gin => Some("Gin"),
                    sql::postgres::SqlIndexAlgorithm::SpGist => Some("SpGist"),
                    sql::postgres::SqlIndexAlgorithm::Brin => Some("Brin"),
                }
            }
            // For views, we copy whatever is written in PSL.
            (None, Some(algo)) => match algo {
                psl::parser_database::IndexAlgorithm::BTree => None,
                psl::parser_database::IndexAlgorithm::Hash => Some("Hash"),
                psl::parser_database::IndexAlgorithm::Gist => Some("Gist"),
                psl::parser_database::IndexAlgorithm::Gin => Some("Gin"),
                psl::parser_database::IndexAlgorithm::SpGist => Some("SpGist"),
                psl::parser_database::IndexAlgorithm::Brin => Some("Brin"),
            },
            _ => None,
        }
    }

    /// The fields that are defining the index.
    pub(crate) fn fields(self) -> Box<dyn Iterator<Item = IndexFieldPair<'a>> + 'a> {
        match (self.next, self.previous) {
            (Some(next), _) => {
                let iter = next.columns().enumerate().map(move |(i, next)| {
                    let previous = self
                        .previous
                        .and_then(|p| p.fields().nth(i).and_then(|f| f.as_scalar_field()));

                    IntrospectionPair::new(self.context, previous, Some(next))
                });

                Box::new(iter)
            }
            (None, Some(prev)) => {
                let iter = prev
                    .fields()
                    .filter_map(|f| f.as_scalar_field())
                    .map(move |prev| IntrospectionPair::new(self.context, Some(prev), None));

                Box::new(iter)
            }
            _ => Box::new(std::iter::empty()),
        }
    }

    /// If one field defines the index, returns that field.
    pub(crate) fn field(self) -> Option<IndexFieldPair<'a>> {
        self.defined_in_a_field().then(|| self.fields().next().unwrap())
    }

    /// True, if we add a new index that has non-default deferring.
    pub(crate) fn adds_a_non_default_deferring(self) -> bool {
        match (self.previous, self.next) {
            (None, Some(next)) => self
                .context
                .flavour
                .uses_non_default_index_deferring(self.context, next),
            _ => false,
        }
    }

    fn defined_in_a_field(self) -> bool {
        if !matches!(self.index_type(), sql::IndexType::Unique) {
            return false;
        }

        match (self.next, self.previous) {
            (Some(next), _) => next.columns().len() == 1,
            (_, Some(prev)) => prev.fields().len() == 1,
            _ => false,
        }
    }
}
