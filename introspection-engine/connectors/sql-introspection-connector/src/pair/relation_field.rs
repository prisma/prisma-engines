use crate::{
    datamodel_calculator::InputContext,
    pair::{ModelPair, Pair},
};
use psl::{
    datamodel_connector::constraint_names::ConstraintNames,
    parser_database::walkers::{self, RelationName},
};
use sql_schema_describer as sql;
use std::borrow::Cow;

/// Defines the direction a relation field is pointing at.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RelationFieldDirection {
    /// The side that defines the foreign key for inlined relations,
    /// and the side A for many to many.
    Forward,
    /// The side that is purely virtual in the PSL, so the client can
    /// access the data from the side that holds no foreign keys, or
    /// the side B for many to many.
    Back,
}

impl RelationFieldDirection {
    fn is_forward(self) -> bool {
        matches!(self, Self::Forward)
    }
}

/// A relation that has a foreign key in the _visible_ Prisma models,
/// combined with a possible existing relation field in the PSL.
#[derive(Clone, Copy)]
struct InlineRelationField<'a> {
    previous: Option<walkers::RelationFieldWalker<'a>>,
    next: sql::ForeignKeyWalker<'a>,
    direction: RelationFieldDirection,
}

impl<'a> InlineRelationField<'a> {
    fn any_field_required(self) -> bool {
        self.next.constrained_columns().any(|col| col.arity().is_required())
    }

    fn any_field_optional(self) -> bool {
        self.next.constrained_columns().any(|col| !col.arity().is_required())
    }

    fn model(self, context: InputContext<'a>) -> ModelPair<'a> {
        let previous = self.previous.map(|prev| prev.model());
        let next = self.next.table();

        Pair::new(context, previous, next)
    }

    fn referenced_model(self, context: InputContext<'a>) -> ModelPair<'a> {
        let previous = self.previous.map(|prev| prev.related_model());
        let next = self.next.referenced_table();

        Pair::new(context, previous, next)
    }

    fn default_constraint_name(self, context: InputContext<'a>) -> String {
        let connector = context.active_connector();
        let cols: Vec<_> = self.next.constrained_columns().map(|c| c.name()).collect();
        ConstraintNames::foreign_key_constraint_name(self.next.table().name(), &cols, connector)
    }
}

/// A relation that holds a hidden join table not visible in the PSL.
/// The foreign key is the one pointing from that table to the
/// referenced model, and which can be used to define the field, type
/// and relation names.
#[derive(Clone, Copy)]
struct Many2ManyRelationField<'a> {
    next: sql::ForeignKeyWalker<'a>,
    /// Forward: model A to B, back: model B to A.
    direction: RelationFieldDirection,
}

/// A field defined in the PSL, when the foreign keys are not enabled.
/// We'll copy these over during introspection.
#[derive(Clone, Copy)]
struct EmulatedRelationField<'a> {
    previous: walkers::RelationFieldWalker<'a>,
}

#[derive(Clone, Copy)]
enum RelationType<'a> {
    Inline(InlineRelationField<'a>),
    Many2Many(Many2ManyRelationField<'a>),
    Emulated(EmulatedRelationField<'a>),
}

#[derive(Clone, Copy)]
pub(crate) struct RelationFieldPair<'a> {
    relation_type: RelationType<'a>,
    context: InputContext<'a>,
}

impl<'a> RelationFieldPair<'a> {
    /// Create a new inline relation field to the wanted direction.
    pub(crate) fn inline(
        context: InputContext<'a>,
        previous: Option<walkers::RelationFieldWalker<'a>>,
        next: sql::ForeignKeyWalker<'a>,
        direction: RelationFieldDirection,
    ) -> Self {
        let relation_type = InlineRelationField {
            previous,
            next,
            direction,
        };

        Self {
            relation_type: RelationType::Inline(relation_type),
            context,
        }
    }

    /// Create a new many to many relation field to the wanted
    /// direction.
    pub(crate) fn m2m(
        context: InputContext<'a>,
        next: sql::ForeignKeyWalker<'a>,
        direction: RelationFieldDirection,
    ) -> Self {
        let relation_type = Many2ManyRelationField { next, direction };

        Self {
            relation_type: RelationType::Many2Many(relation_type),
            context,
        }
    }

    /// Create a new emulated relation field, if using `relationMode`
    /// `prisma`.
    pub(crate) fn emulated(context: InputContext<'a>, previous: walkers::RelationFieldWalker<'a>) -> Self {
        let relation_type = EmulatedRelationField { previous };

        Self {
            relation_type: RelationType::Emulated(relation_type),
            context,
        }
    }

    /// The name of the relation field.
    pub(crate) fn field_name(self) -> &'a str {
        use RelationType::*;

        match self.relation_type {
            Inline(field) if field.direction.is_forward() => {
                self.context.forward_inline_relation_field_prisma_name(field.next.id)
            }
            Inline(field) => self.context.back_inline_relation_field_prisma_name(field.next.id),
            Many2Many(field) if field.direction.is_forward() => self
                .context
                .forward_m2m_relation_field_prisma_name(field.next.table().id),
            Many2Many(field) => self.context.back_m2m_relation_field_prisma_name(field.next.table().id),
            Emulated(field) => field.previous.name(),
        }
    }

    /// The Prisma type of the relation field.
    pub(crate) fn prisma_type(self) -> Cow<'a, str> {
        use RelationType::*;

        match self.relation_type {
            Inline(field) if field.direction.is_forward() => {
                let id = field.next.referenced_table().id;
                self.context.table_prisma_name(id).prisma_name()
            }
            Inline(field) => {
                let id = field.next.table().id;
                self.context.table_prisma_name(id).prisma_name()
            }
            Many2Many(field) => {
                let id = field.next.referenced_table().id;
                self.context.table_prisma_name(id).prisma_name()
            }
            Emulated(field) => {
                let name = field.previous.related_model().name();
                Cow::Borrowed(name)
            }
        }
    }

    /// The name of the foreign key constraint, if using foreign keys
    /// and the constraint name is non-standard.
    pub(crate) fn constraint_name(self) -> Option<&'a str> {
        match self.relation_type {
            RelationType::Inline(field) if field.direction.is_forward() => {
                if let Some(name) = field.previous.and_then(|prev| prev.mapped_name()) {
                    return Some(name);
                }

                let default_name = field.default_constraint_name(self.context);
                field.next.constraint_name().filter(|name| name != &default_name)
            }
            RelationType::Emulated(field) => field.previous.mapped_name(),
            _ => None,
        }
    }

    /// The name of the relation, if needed for disambiguation.
    pub(crate) fn relation_name(self) -> Option<Cow<'a, str>> {
        let name = match self.relation_type {
            RelationType::Inline(field) => self.context.inline_relation_prisma_name(field.next.id),
            RelationType::Many2Many(field) => self.context.m2m_relation_prisma_name(field.next.table().id),
            RelationType::Emulated(field) => match field.previous.relation_name() {
                RelationName::Explicit(name) => Cow::Borrowed(name),
                RelationName::Generated(_) => Cow::Borrowed(""),
            },
        };

        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }

    /// The referencing fields in the current model.
    pub(crate) fn fields(self) -> Option<Box<dyn Iterator<Item = Cow<'a, str>> + 'a>> {
        match self.relation_type {
            RelationType::Inline(field) if field.direction.is_forward() => {
                let iter = field
                    .next
                    .constrained_columns()
                    .map(move |c| self.context.column_prisma_name(c.id).prisma_name());

                let iter: Box<dyn Iterator<Item = Cow<'a, str>>> = Box::new(iter);
                Some(iter)
            }
            RelationType::Emulated(field) => field.previous.referencing_fields().map(|f| {
                let iter = Box::new(f.map(|f| Cow::Borrowed(f.name())));
                iter as Box<dyn Iterator<Item = Cow<'a, str>>>
            }),
            _ => None,
        }
    }

    /// The referenced fiends in the other model.
    pub(crate) fn references(self) -> Option<Box<dyn Iterator<Item = Cow<'a, str>> + 'a>> {
        match self.relation_type {
            RelationType::Inline(field) if field.direction.is_forward() => {
                let iter = field
                    .next
                    .referenced_columns()
                    .map(move |c| self.context.column_prisma_name(c.id).prisma_name());

                let iter: Box<dyn Iterator<Item = Cow<'a, str>>> = Box::new(iter);
                Some(iter)
            }
            RelationType::Emulated(field) => field.previous.referenced_fields().map(|f| {
                let iter = Box::new(f.map(|f| Cow::Borrowed(f.name())));
                iter as Box<dyn Iterator<Item = Cow<'a, str>>>
            }),
            _ => None,
        }
    }

    /// The `onDelete` referential action, if non-default.
    pub(crate) fn on_delete(self) -> Option<&'a str> {
        match self.relation_type {
            RelationType::Inline(field) if field.direction.is_forward() => {
                use sql::ForeignKeyAction::*;

                match (field.any_field_required(), field.next.on_delete_action()) {
                    (false, SetNull) => None,
                    (true, Restrict) => None,
                    (true, NoAction) if self.context.sql_family.is_mssql() => None,
                    (_, Cascade) => Some("Cascade"),
                    (_, SetDefault) => Some("SetDefault"),
                    (true, SetNull) => Some("SetNull"),
                    (_, NoAction) => Some("NoAction"),
                    (false, Restrict) => Some("Restrict"),
                }
            }
            RelationType::Emulated(field) => field.previous.explicit_on_delete().map(|act| act.as_str()),
            _ => None,
        }
    }

    /// The `onUpdate` referential action, if non-default.
    pub(crate) fn on_update(self) -> Option<&'a str> {
        match self.relation_type {
            RelationType::Inline(field) if field.direction.is_forward() => {
                use sql::ForeignKeyAction::*;

                match field.next.on_update_action() {
                    Cascade => None,
                    NoAction => Some("NoAction"),
                    Restrict => Some("Restrict"),
                    SetNull => Some("SetNull"),
                    SetDefault => Some("SetDefault"),
                }
            }
            RelationType::Emulated(field) => field.previous.explicit_on_update().map(|act| act.as_str()),
            _ => None,
        }
    }

    /// If the field should be ignored.
    pub(crate) fn ignore(self) -> bool {
        use RelationFieldDirection::*;

        match self.relation_type {
            RelationType::Inline(field) => {
                let missing_identifiers = !table_has_usable_identifier(field.next.table())
                    || !table_has_usable_identifier(field.next.referenced_table());

                let model_ignored = match field.direction {
                    Forward => field.model(self.context).ignored(),
                    Back => field.referenced_model(self.context).ignored(),
                };

                missing_identifiers && !model_ignored
            }
            RelationType::Many2Many(_) => false,
            RelationType::Emulated(field) => field.previous.is_ignored(),
        }
    }

    /// If we should render the `@relation` attribute to the field.
    pub(crate) fn renders_attribute(self) -> bool {
        match self.relation_type {
            RelationType::Inline(field) if field.direction.is_forward() => true,
            RelationType::Emulated(field) => field.previous.relation_attribute().is_some(),
            _ => self.relation_name().is_some(),
        }
    }

    /// Is the relation field optional.
    pub(crate) fn is_optional(self) -> bool {
        match self.relation_type {
            RelationType::Inline(field) if field.direction.is_forward() => field.any_field_optional(),
            RelationType::Inline(field) => forward_relation_field_is_unique(field.next),
            RelationType::Emulated(field) => field.previous.ast_field().arity.is_optional(),
            RelationType::Many2Many(_) => false,
        }
    }

    /// Is the relation field an array.
    pub(crate) fn is_array(self) -> bool {
        match self.relation_type {
            RelationType::Inline(field) if field.direction.is_forward() => false,
            RelationType::Inline(field) => !forward_relation_field_is_unique(field.next),
            RelationType::Emulated(field) => field.previous.ast_field().arity.is_list(),
            RelationType::Many2Many(_) => true,
        }
    }

    /// If the relation is completely taken from the PSL.
    pub(crate) fn reintrospected_relation(self) -> bool {
        matches!(self.relation_type, RelationType::Emulated(_))
    }
}

fn forward_relation_field_is_unique(fk: sql::ForeignKeyWalker) -> bool {
    fk.table()
        .indexes()
        .filter(|idx| idx.is_primary_key() || idx.is_unique())
        .any(|idx| {
            idx.columns().all(|idx_col| {
                fk.constrained_columns()
                    .any(|fk_col| fk_col.id == idx_col.as_column().id)
            })
        })
}

fn table_has_usable_identifier(table: sql::TableWalker<'_>) -> bool {
    table
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
