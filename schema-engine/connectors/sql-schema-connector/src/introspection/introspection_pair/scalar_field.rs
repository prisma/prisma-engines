use crate::introspection::sanitize_datamodel_names;
use either::Either;
use psl::{
    datamodel_connector::walker_ext_traits::IndexWalkerExt, parser_database::walkers,
    schema_ast::ast::WithDocumentation,
};
use sql::ColumnArity;
use sql_schema_describer as sql;
use std::borrow::Cow;

use super::{DefaultValuePair, IdPair, IndexPair, IntrospectionPair};

/// Comparing a possible previous PSL scalar field
/// to a column from the database. Re-introspection
/// can use some of the previous values in the new
/// rendering.
pub(crate) type ScalarFieldPair<'a> =
    IntrospectionPair<'a, Option<walkers::ScalarFieldWalker<'a>>, sql::ColumnWalker<'a>>;

impl<'a> ScalarFieldPair<'a> {
    /// The client name of the field.
    pub fn name(self) -> Cow<'a, str> {
        let name = self.context.column_prisma_name(self.next.id).prisma_name();

        if name.is_empty() {
            Cow::Borrowed(self.next.name())
        } else {
            name
        }
    }

    /// How the field is named in the database, if different than the
    /// client name.
    pub fn mapped_name(self) -> Option<&'a str> {
        self.context.column_prisma_name(self.next.id).mapped_name()
    }

    /// If the field acts as an updated at column.
    pub fn is_updated_at(self) -> bool {
        self.previous.map(|f| f.is_updated_at()).unwrap_or(false)
    }

    /// If the field is ignored in the client.
    pub fn is_ignored(self) -> bool {
        self.previous.map(|f| f.is_ignored()).unwrap_or(false)
    }

    /// True if we took the name from the PSL.
    pub(crate) fn remapped_name_from_psl(&self) -> bool {
        self.previous.and_then(|p| p.mapped_name()).is_some()
    }

    /// True if we cannot sanitize the given name.
    pub(crate) fn remapped_name_empty(&self) -> bool {
        sanitize_datamodel_names::sanitize_string(self.next.name()).is_empty()
    }

    /// The documentation block of the field from PSL.
    pub(crate) fn documentation(&self) -> Option<&'a str> {
        self.previous.and_then(|f| f.ast_field().documentation())
    }

    /// Optional, required or a list.
    pub fn arity(self) -> ColumnArity {
        if self.context.flavour.keep_previous_scalar_field_arity(self.next) {
            match self.previous.map(|prev| prev.ast_field().arity) {
                Some(arity) if arity.is_required() => ColumnArity::Required,
                Some(arity) if arity.is_list() => ColumnArity::List,
                _ => self.next.column_type().arity,
            }
        } else {
            self.next.column_type().arity
        }
    }

    /// If we cannot support the field type in the client.
    pub fn is_unsupported(self) -> bool {
        self.next.column_type_family().is_unsupported()
    }

    /// The client type.
    pub fn prisma_type(self) -> Cow<'a, str> {
        match self.column_type_family() {
            sql::ColumnTypeFamily::Int => Cow::from("Int"),
            sql::ColumnTypeFamily::BigInt => Cow::from("BigInt"),
            sql::ColumnTypeFamily::Float => Cow::from("Float"),
            sql::ColumnTypeFamily::Decimal => Cow::from("Decimal"),
            sql::ColumnTypeFamily::Boolean => Cow::from("Boolean"),
            sql::ColumnTypeFamily::String => Cow::from("String"),
            sql::ColumnTypeFamily::DateTime => Cow::from("DateTime"),
            sql::ColumnTypeFamily::Binary => Cow::from("Bytes"),
            sql::ColumnTypeFamily::Json => Cow::from("Json"),
            sql::ColumnTypeFamily::Uuid => Cow::from("String"),
            sql::ColumnTypeFamily::Enum(id) => self.context.enum_prisma_name(*id).prisma_name(),
            sql::ColumnTypeFamily::Unsupported(ref typ) => Cow::from(typ),
        }
    }

    /// The database type, if non-default.
    pub fn native_type(self) -> Option<(&'a str, &'static str, Vec<String>)> {
        let scalar_type = match self.column_type_family() {
            sql::ColumnTypeFamily::Int => Some(psl::parser_database::ScalarType::Int),
            sql::ColumnTypeFamily::BigInt => Some(psl::parser_database::ScalarType::BigInt),
            sql::ColumnTypeFamily::Float => Some(psl::parser_database::ScalarType::Float),
            sql::ColumnTypeFamily::Decimal => Some(psl::parser_database::ScalarType::Decimal),
            sql::ColumnTypeFamily::Boolean => Some(psl::parser_database::ScalarType::Boolean),
            sql::ColumnTypeFamily::String => Some(psl::parser_database::ScalarType::String),
            sql::ColumnTypeFamily::DateTime => Some(psl::parser_database::ScalarType::DateTime),
            sql::ColumnTypeFamily::Json => Some(psl::parser_database::ScalarType::Json),
            sql::ColumnTypeFamily::Uuid => Some(psl::parser_database::ScalarType::String),
            sql::ColumnTypeFamily::Binary => Some(psl::parser_database::ScalarType::Bytes),
            sql::ColumnTypeFamily::Enum(_) => None,
            sql::ColumnTypeFamily::Unsupported(_) => None,
        };

        let native_type = self.next.column_type().native_type.as_ref();

        if let Some((scalar_type, native_type)) = scalar_type.and_then(|st| native_type.map(|nt| (st, nt))) {
            let is_default = self
                .context
                .active_connector()
                .native_type_is_default_for_scalar_type(native_type, &scalar_type);

            if is_default {
                None
            } else {
                let (r#type, params) = self.context.active_connector().native_type_to_parts(native_type);
                let prefix = &self.context.config.datasources.first().unwrap().name;

                Some((prefix, r#type, params))
            }
        } else {
            None
        }
    }

    /// The primary key of the field.
    pub fn id(self) -> Option<IdPair<'a>> {
        match self.next.refine() {
            // Only rendering for tables, if having the primary key in the database.
            Either::Left(table_col) => table_col
                .table()
                .primary_key()
                .filter(|pk| pk.columns().len() == 1)
                .filter(|pk| pk.contains_column(table_col.id))
                .map(move |next| {
                    let previous = self.previous.and_then(|field| field.model().primary_key());
                    IntrospectionPair::new(self.context, previous, Some(next))
                }),
            // Rendering the id for views, if user has explicitly written it in PSL.
            Either::Right(_) => self
                .previous
                .and_then(|prev| prev.model().primary_key().map(|pk| (prev, pk)))
                .filter(|(prev, pk)| pk.contains_exactly_fields(std::iter::once(*prev)))
                .map(|(_, pk)| IntrospectionPair::new(self.context, Some(pk), None)),
        }
    }

    /// If the field itself defines a unique constraint.
    pub fn unique(self) -> Option<IndexPair<'a>> {
        match self.next.refine() {
            Either::Left(table_col) => {
                let next = table_col
                    .table()
                    .indexes()
                    .filter(|i| i.is_unique())
                    .filter(|i| i.columns().len() == 1)
                    .find(|i| i.contains_column(table_col.id));

                next.map(move |next| {
                    let previous = self.previous.and_then(|field| {
                        field.model().indexes().find(|idx| {
                            // Upgrade logic. Prior to Prisma 3, PSL index attributes had a `name` argument but no `map`
                            // argument. If we infer that an index in the database was produced using that logic, we
                            // match up the existing index.
                            if idx.mapped_name().is_none() && idx.name() == Some(next.name()) {
                                return true;
                            }

                            // Compare the constraint name (implicit or mapped name) from the Prisma schema with the
                            // constraint name from the database.
                            idx.constraint_name(self.context.active_connector()) == next.name()
                        })
                    });

                    IntrospectionPair::new(self.context, previous, Some(next))
                })
            }
            // A view column is unique, if explicitly defined in PSL.
            Either::Right(_) => self.previous.and_then(move |prev| {
                prev.model()
                    .indexes()
                    .filter(|idx| idx.is_unique())
                    .filter(|idx| idx.is_defined_on_field())
                    .filter(|idx| idx.contains_field(prev))
                    .map(|idx| IntrospectionPair::new(self.context, Some(idx), None))
                    .next()
            }),
        }
    }

    /// The default value constraint.
    pub fn default(self) -> DefaultValuePair<'a> {
        let previous = self.previous.and_then(|prev| prev.default_value());
        IntrospectionPair::new(self.context, previous, self.next)
    }

    /// The COMMENT of the field.
    pub(crate) fn description(self) -> Option<&'a str> {
        self.next.description()
    }

    /// True if we have a new field and it has a comment.
    pub(crate) fn adds_a_description(self) -> bool {
        self.previous.is_none() && self.description().is_some()
    }

    fn column_type_family(self) -> &'a sql::ColumnTypeFamily {
        self.next.column_type_family()
    }
}
