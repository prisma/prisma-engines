use psl::{
    datamodel_connector::walker_ext_traits::IndexWalkerExt,
    parser_database::{self as db, walkers},
    schema_ast::ast::WithDocumentation,
};
use sql::postgres::PostgresSchemaExt;
use sql_schema_describer as sql;
use std::borrow::Cow;

use super::{IdPair, IndexPair, IntrospectionPair, RelationFieldDirection, RelationFieldPair, ScalarFieldPair};

/// Comparing a possible PSL model definition
/// to a table in a database. For re-introspection
/// some values will be copied from the previons
/// data model.
pub(crate) type ModelPair<'a> = IntrospectionPair<'a, Option<walkers::ModelWalker<'a>>, sql::TableWalker<'a>>;

impl<'a> ModelPair<'a> {
    /// The position of the model from the PSL, if existing. Used for
    /// sorting the models in the final introspected data model.
    pub(crate) fn previous_position(self) -> Option<db::ModelId> {
        self.previous.map(|m| m.id)
    }

    /// Temporary method for relations. Eventually we'll remove this
    /// when we handle relations together with models and fields.
    pub(crate) fn table_id(self) -> sql::TableId {
        self.next.id
    }

    /// The namespace of the model, if using the multi-schema feature.
    pub(crate) fn namespace(self) -> Option<&'a str> {
        self.context.uses_namespaces().then(|| self.next.namespace()).flatten()
    }

    /// Whether the model is a partition table or not.
    pub(crate) fn is_partition(self) -> bool {
        self.next.is_partition()
    }

    /// True, if we add a new model with a partition.
    pub(crate) fn new_with_partition(self) -> bool {
        self.previous.is_none() && self.is_partition()
    }

    /// Whether the model has subclass tables or not.
    pub(crate) fn has_subclass(self) -> bool {
        self.next.has_subclass()
    }

    /// True, if we add a new model with a subclass.
    pub(crate) fn new_with_subclass(self) -> bool {
        self.previous.is_none() && self.has_subclass()
    }

    /// Whether the model has row level security enabled.
    pub(crate) fn has_row_level_security(self) -> bool {
        self.next.has_row_level_security()
    }

    /// Whether the model has check constraints.
    pub(crate) fn adds_check_constraints(self) -> bool {
        self.previous.is_none() && self.next.has_check_constraints()
    }

    /// The names of check constraints for this model.
    pub(crate) fn check_constraints(self) -> impl Iterator<Item = &'a str> {
        self.next.check_constraints()
    }

    /// Whether the model has exclusion constraints.
    pub(crate) fn adds_exclusion_constraints(self) -> bool {
        self.previous.is_none() && self.context.flavour.uses_exclude_constraint(self.context, self.next)
    }

    pub(crate) fn expression_indexes(self) -> impl Iterator<Item = &'a str> {
        let mut indexes = None;
        if self.context.sql_family().is_postgres() {
            let data: &PostgresSchemaExt = self.context.sql_schema.downcast_connector_data();

            indexes = Some(
                data.expression_indexes
                    .iter()
                    .filter(move |(table_id, _idx)| *table_id == self.next.id)
                    .map(|(_table_id, idx)| idx.as_str()),
            );
        }

        indexes.into_iter().flatten()
    }

    pub(crate) fn include_indexes(self) -> impl Iterator<Item = &'a str> {
        let mut indexes = None;
        if self.context.sql_family().is_postgres() {
            let data: &PostgresSchemaExt = self.context.sql_schema.downcast_connector_data();

            indexes = Some(
                data.include_indexes
                    .iter()
                    .filter(move |(table_id, _idx)| *table_id == self.next.id)
                    .map(|(_table_id, idx)| idx.as_str()),
            );
        }

        indexes.into_iter().flatten()
    }

    /// True, if we add a new model with row level security enabled.
    pub(crate) fn adds_row_level_security(self) -> bool {
        self.previous.is_none() && self.has_row_level_security()
    }

    /// True, if we add an index with non-default null position.
    pub(crate) fn adds_non_default_null_position(self) -> bool {
        self.all_indexes()
            .flat_map(|i| i.fields())
            .any(|f| f.adds_non_default_null_position())
    }

    /// Name of the model in the PSL. The value can be sanitized if it
    /// contains characters that are not allowed in the PSL
    /// definition.
    pub(crate) fn name(self) -> Cow<'a, str> {
        self.context.table_prisma_name(self.next.id).prisma_name()
    }

    /// The mapped name, if defined, is the actual name of the model in
    /// the database.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        self.context.table_prisma_name(self.next.id).mapped_name()
    }

    /// True, if the name of the model is using a reserved identifier.
    /// If we already have a model in the PSL, the validation will not
    /// allow reserved names and we don't need to warn the user.
    pub(crate) fn uses_reserved_name(self) -> bool {
        psl::is_reserved_type_name(self.next.name()) && self.previous.is_none()
    }

    /// The documentation on top of the Model.
    pub(crate) fn documentation(self) -> Option<&'a str> {
        self.previous.and_then(|model| model.ast_model().documentation())
    }

    /// Iterating over the scalar fields.
    pub(crate) fn scalar_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldPair<'a>> {
        self.next.columns().map(move |next| {
            let previous = self.context.existing_table_scalar_field(next.id);
            IntrospectionPair::new(self.context, previous, next.coarsen())
        })
    }

    /// Iterating over the relation fields.
    pub(crate) fn relation_fields(self) -> Box<dyn Iterator<Item = RelationFieldPair<'a>> + 'a> {
        if self.context.foreign_keys_enabled() {
            let inline = self
                .context
                .inline_relations_for_table(self.table_id())
                .map(move |(direction, fk)| {
                    let previous = self
                        .context
                        .existing_inline_relation(fk.id)
                        .and_then(|rel| match direction {
                            RelationFieldDirection::Forward => rel.forward_relation_field(),
                            RelationFieldDirection::Back => rel.back_relation_field(),
                        });

                    RelationFieldPair::inline(self.context, previous, fk, direction)
                });

            let m2m = self
                .context
                .m2m_relations_for_table(self.table_id())
                .map(move |(direction, next)| RelationFieldPair::m2m(self.context, next, direction));

            match self.previous {
                Some(prev) => {
                    // View relations are currently a bit special.
                    // We do not have foreign keys that point to or start
                    // from a view. The client needs the relations to do
                    // joins, so now we just copy them from the PSL
                    // in re-introspection.
                    let view_relations = prev
                        .relation_fields()
                        .filter(|rf| rf.one_side_is_view())
                        .filter(move |rf| !self.context.table_missing_for_model(&rf.related_model().id))
                        .filter(move |rf| !self.context.view_missing_for_model(&rf.related_model().id))
                        .map(move |previous| RelationFieldPair::emulated(self.context, previous));

                    Box::new(inline.chain(m2m).chain(view_relations))
                }
                None => Box::new(inline.chain(m2m)),
            }
        } else {
            match self.previous {
                Some(prev) => {
                    // If not using foreign keys, the relation fields
                    // are copied from the previous PSL.
                    let fields = prev
                        .relation_fields()
                        .filter(move |rf| !self.context.table_missing_for_model(&rf.related_model().id))
                        .map(move |previous| RelationFieldPair::emulated(self.context, previous));

                    Box::new(fields)
                }
                None => Box::new(std::iter::empty()),
            }
        }
    }

    /// True, if the user has explicitly mapped the model's name in
    /// the PSL.
    pub(crate) fn remapped_name(self) -> bool {
        self.previous.filter(|m| m.mapped_name().is_some()).is_some()
    }

    /// True, if we have a new model that uses row level TTL.
    pub(crate) fn adds_a_row_level_ttl(self) -> bool {
        self.previous.is_none() && self.context.flavour.uses_row_level_ttl(self.context, self.next)
    }

    /// True, if we _add_ a new constraint with a non-default
    /// deferring.
    pub(crate) fn adds_non_default_deferring(self) -> bool {
        let from_index = self.all_indexes().any(|i| i.adds_a_non_default_deferring());

        let from_fk = self
            .relation_fields()
            .filter(|rf| rf.fields().is_some())
            .any(|rf| rf.adds_non_default_deferring());

        self.previous.is_none() && (from_index || from_fk)
    }

    /// A model must have either a primary key, or at least one unique
    /// index defined that consists of columns that are all supported by
    /// prisma and not null.
    pub(crate) fn has_usable_identifier(self) -> bool {
        self.next
            .indexes()
            .filter(|idx| idx.is_primary_key() || idx.is_unique())
            .any(|idx| {
                idx.columns().all(|c| {
                    !matches!(
                        c.as_column().column_type().family,
                        sql::ColumnTypeFamily::Unsupported(_)
                    ) && c.as_column().arity().is_required()
                })
            })
    }

    /// True, if the model uses the same name as another top-level item from
    /// a different namespace.
    pub(crate) fn uses_duplicate_name(self) -> bool {
        self.previous.is_none() && !self.context.name_is_unique(self.next.name())
    }

    /// If the model is marked as ignored. Can happen either if user
    /// explicitly sets the model attribute, or if the model has no
    /// usable identifiers.
    pub(crate) fn ignored(self) -> bool {
        let explicit_ignore = self.ignored_in_psl();
        let implicit_ignore = !self.has_usable_identifier() && self.scalar_fields().len() > 0;

        explicit_ignore || implicit_ignore
    }

    /// If the model is already marked as ignored in the PSL.
    pub(crate) fn ignored_in_psl(self) -> bool {
        self.previous.map(|model| model.is_ignored()).unwrap_or(false)
    }

    /// Returns an iterator over all indexes of the model,
    /// specifically the ones defined in the model level, skipping the
    /// primary key and unique index defined in a field.
    ///
    /// For the primary key, use [`ModelPair#id`]. For a field-level
    /// unique, use [`ScalarFieldPair#unique`].
    pub(crate) fn indexes(self) -> impl Iterator<Item = IndexPair<'a>> {
        self.next
            .indexes()
            .filter(|i| !(i.is_unique() && i.columns().len() == 1))
            .filter(|i| !i.is_primary_key())
            .map(move |next| {
                let previous = self.previous.and_then(|prev| {
                    prev.indexes().find(|idx| {
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

    /// The primary key of the model, if defined. It will only return
    /// a value, if the field should be defined in a model as `@@id`:
    /// e.g. when it holds more than one field.
    pub(crate) fn id(self) -> Option<IdPair<'a>> {
        self.next
            .primary_key()
            .filter(|pk| pk.columns().len() > 1)
            .and_then(move |pk| {
                let id = self.previous.and_then(|model| model.primary_key());
                let pair = IntrospectionPair::new(self.context, id, Some(pk));

                (!pair.defined_in_a_field()).then_some(pair)
            })
    }

    /// The COMMENT of the model.
    pub(crate) fn description(self) -> Option<&'a str> {
        self.next.description()
    }

    /// True if we have a new model and it has a comment.
    pub(crate) fn adds_a_description(self) -> bool {
        self.previous.is_none()
            && (self.description().is_some() || self.scalar_fields().any(|sf| sf.adds_a_description()))
    }

    fn all_indexes(self) -> impl ExactSizeIterator<Item = IndexPair<'a>> {
        self.next.indexes().map(move |next| {
            let previous = self.previous.and_then(|prev| {
                prev.indexes().find(|idx| {
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
}
