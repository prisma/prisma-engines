use super::{IdPair, IndexPair, IntrospectionPair, RelationFieldPair, ScalarFieldPair};
use psl::{
    parser_database::{self as db, walkers},
    schema_ast::ast::WithDocumentation,
};
use sql_schema_describer as sql;
use std::borrow::Cow;

/// Comparing a PSL view (which currently utilizes the
/// model structure due to them being completely the same
/// things) to a database view.
pub(crate) type ViewPair<'a> = IntrospectionPair<'a, Option<walkers::ModelWalker<'a>>, sql::ViewWalker<'a>>;

impl<'a> ViewPair<'a> {
    /// The position of the view from the PSL, if existing. Used for
    /// sorting the views in the final introspected data model.
    pub(crate) fn previous_position(self) -> Option<db::ModelId> {
        self.previous.map(|m| m.id)
    }

    /// The namespace of the view, if using the multi-schema feature.
    pub(crate) fn namespace(self) -> Option<&'a str> {
        self.context.uses_namespaces().then(|| self.next.namespace()).flatten()
    }

    /// Name of the view in the PSL. The value can be sanitized if it
    /// contains characters that are not allowed in the PSL
    /// definition.
    pub(crate) fn name(self) -> Cow<'a, str> {
        self.context.view_prisma_name(self.next.id).prisma_name()
    }

    /// The mapped name, if defined, is the actual name of the view in
    /// the database.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        self.context.view_prisma_name(self.next.id).mapped_name()
    }

    /// True, if the name of the view is using a reserved identifier.
    /// If we already have a view in the PSL, the validation will not
    /// allow reserved names and we don't need to warn the user.
    pub(crate) fn uses_reserved_name(self) -> bool {
        psl::is_reserved_type_name(self.next.name()) && self.previous.is_none()
    }

    /// The documentation on top of the view.
    pub(crate) fn documentation(self) -> Option<&'a str> {
        self.previous.and_then(|view| view.ast_model().documentation())
    }

    /// Iterating over the scalar fields.
    pub(crate) fn scalar_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldPair<'a>> {
        self.next.columns().map(move |next| {
            let previous = self.context.existing_view_scalar_field(next.id);
            IntrospectionPair::new(self.context, previous, next.coarsen())
        })
    }

    /// Iterating over the relation fields.
    pub(crate) fn relation_fields(self) -> Box<dyn Iterator<Item = RelationFieldPair<'a>> + 'a> {
        match self.previous {
            Some(prev) => {
                let iter = prev
                    .relation_fields()
                    .filter(move |rf| !self.context.table_missing_for_model(&rf.related_model().id))
                    .filter(move |rf| !self.context.view_missing_for_model(&rf.related_model().id))
                    .map(move |prev| RelationFieldPair::emulated(self.context, prev));

                Box::new(iter)
            }
            None => Box::new(std::iter::empty()),
        }
    }

    /// True, if the user has explicitly mapped the view's name in
    /// the PSL.
    pub(crate) fn remapped_name(self) -> bool {
        self.previous.filter(|v| v.mapped_name().is_some()).is_some()
    }

    /// A view must have either an id, or at least one unique
    /// index defined that consists of columns that are all supported by
    /// prisma and not null.
    ///
    /// We cannot fetch these from the underlying table during introspection,
    /// so this is always false if the user hasn't explicitly specified them
    /// in the PSL.
    pub(crate) fn has_usable_identifier(self) -> bool {
        let identifier_in_indices = self
            .previous
            .map(|view| view.indexes().filter(|idx| idx.is_unique()))
            .map(|mut idxs| {
                idxs.any(|idx| {
                    idx.fields()
                        .all(|f| !f.is_unsupported() && f.ast_field().arity.is_required())
                })
            })
            .unwrap_or(false);

        let identifier_in_id = self
            .previous
            .and_then(|view| {
                view.primary_key().map(|pk| {
                    pk.fields()
                        .all(|f| !f.is_unsupported() && f.ast_field().arity.is_required())
                })
            })
            .unwrap_or(false);

        identifier_in_indices || identifier_in_id
    }

    /// True, if the view uses the same name as another top-level item from
    /// a different namespace.
    pub(crate) fn uses_duplicate_name(self) -> bool {
        self.previous.is_none() && !self.context.name_is_unique(self.next.name())
    }

    /// If the view is marked as ignored. Can happen either if user
    /// explicitly sets the view attribute, or if the view has no
    /// usable identifiers.
    pub(crate) fn ignored(self) -> bool {
        let explicit_ignore = self.ignored_in_psl();
        let implicit_ignore = !self.has_usable_identifier() && self.scalar_fields().len() > 0;

        explicit_ignore || implicit_ignore
    }

    /// If the view is already marked as ignored in the PSL.
    pub(crate) fn ignored_in_psl(self) -> bool {
        self.previous.map(|view| view.is_ignored()).unwrap_or(false)
    }

    /// Returns an iterator over all indexes of the view explicitly defined in PSL.
    ///
    /// For the primary key, use [`ModelPair#id`]. For a field-level
    /// unique, use [`ScalarFieldPair#unique`].
    pub(crate) fn indexes(self) -> Box<dyn Iterator<Item = IndexPair<'a>> + 'a> {
        match self.previous {
            Some(prev) => {
                let iter = prev
                    .indexes()
                    .filter(|i| !(i.is_unique() && i.fields().len() == 1))
                    .map(move |prev| IntrospectionPair::new(self.context, Some(prev), None));

                Box::new(iter)
            }
            None => Box::new(std::iter::empty()),
        }
    }

    /// The primary key of the view, if defined as a block constraint
    /// in the view.
    ///
    /// As with the unique indexes, it is just a virtual thing for
    /// the client to be able to query the view for now.
    ///
    /// The id is always just in the PSL, and we cannot get that
    /// information from the information schema.
    ///
    /// For a field-level id, use [`ScalarFieldPair#unique`].
    pub(crate) fn id(self) -> Option<IdPair<'a>> {
        self.previous
            .and_then(|prev| prev.primary_key())
            .filter(|pk| pk.fields().len() > 1)
            .map(|prev| IntrospectionPair::new(self.context, Some(prev), None))
    }

    /// The SQL definition statement of the view.
    pub(crate) fn definition(self) -> Option<String> {
        self.next
            .definition()
            .map(|s| self.context.flavour.format_view_definition(s))
    }

    /// The COMMENT of the view.
    pub(crate) fn description(self) -> Option<&'a str> {
        self.next.description()
    }

    /// True if we introspect the view for the first time, and it has a comment
    /// in the database.
    pub(crate) fn adds_a_description(self) -> bool {
        self.previous.is_none()
            && (self.description().is_some() || self.scalar_fields().any(|sf| sf.adds_a_description()))
    }
}
