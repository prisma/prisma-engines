use crate::SqlFamilyTrait;
use psl::{
    datamodel_connector::constraint_names::ConstraintNames, parser_database::walkers, schema_ast::ast, PreviewFeature,
};
use sql::{mssql::MssqlSchemaExt, postgres::PostgresSchemaExt};
use sql_schema_describer as sql;

use super::{IndexFieldPair, Pair};

pub(crate) type IndexPair<'a> = Pair<'a, walkers::IndexWalker<'a>, sql::IndexWalker<'a>>;

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
        (self.next.name() != self.default_constraint_name()).then(|| self.next.name())
    }

    /// The type of the index.
    pub(crate) fn index_type(self) -> sql::IndexType {
        let preview_features = self.context.config.preview_features();

        match self.next.index_type() {
            sql::IndexType::Fulltext if !preview_features.contains(PreviewFeature::FullTextIndex) => {
                sql::IndexType::Normal
            }
            typ => typ,
        }
    }

    /// SQL Server specific clustering setting. A value is returned if
    /// non-default.
    pub(crate) fn clustered(self) -> Option<bool> {
        if !self.context.sql_family.is_mssql() {
            return None;
        }

        let ext: &MssqlSchemaExt = self.context.schema.downcast_connector_data();
        let clustered = ext.index_is_clustered(self.next.id);

        if !clustered {
            return None;
        }

        Some(clustered)
    }

    /// A PostgreSQL specific algorithm. Defines the data structure
    /// that defines the index.
    pub(crate) fn algorithm(self) -> Option<&'static str> {
        if !self.context.sql_family().is_postgres() {
            return None;
        }

        let data: &PostgresSchemaExt = self.context.schema.downcast_connector_data();

        match data.index_algorithm(self.next.id) {
            sql::postgres::SqlIndexAlgorithm::BTree => None,
            sql::postgres::SqlIndexAlgorithm::Hash => Some("Hash"),
            sql::postgres::SqlIndexAlgorithm::Gist => Some("Gist"),
            sql::postgres::SqlIndexAlgorithm::Gin => Some("Gin"),
            sql::postgres::SqlIndexAlgorithm::SpGist => Some("SpGist"),
            sql::postgres::SqlIndexAlgorithm::Brin => Some("Brin"),
        }
    }

    /// The fields that are defining the index.
    pub(crate) fn fields(self) -> impl ExactSizeIterator<Item = IndexFieldPair<'a>> {
        self.next.columns().enumerate().map(move |(i, next)| {
            let previous = self
                .previous
                .and_then(|p| p.fields().nth(i).and_then(|f| f.as_scalar_field()));
            Pair::new(self.context, previous, next)
        })
    }

    /// If one field defines the index, returns that field.
    pub(crate) fn field(self) -> Option<IndexFieldPair<'a>> {
        self.defined_in_a_field().then(|| self.fields().next().unwrap())
    }

    fn defined_in_a_field(self) -> bool {
        if !matches!(self.index_type(), sql::IndexType::Unique) {
            return false;
        }

        self.fields().len() == 1
    }

    fn default_constraint_name(self) -> String {
        let columns = self.next.column_names().collect::<Vec<_>>();

        match self.next.index_type() {
            sql::IndexType::Unique => {
                ConstraintNames::unique_index_name(self.next.table().name(), &columns, self.context.active_connector())
            }
            _ => ConstraintNames::non_unique_index_name(
                self.next.table().name(),
                &columns,
                self.context.active_connector(),
            ),
        }
    }
}
