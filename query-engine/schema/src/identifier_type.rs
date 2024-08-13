use crate::{capitalize, constants::ordering, scalar_filter_name};
use query_structure::{ast::FieldArity, prelude::*, *};

/// Enum used to represent unique schema type names.
/// It helps deferring the allocation + formatting of strings
/// during the initialization of the schema, which proved to be very costly on large schemas.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum IdentifierType {
    SortOrder,
    AffectedRowsOutput,
    Query,
    Mutation,
    CheckedCreateInput(Model, Option<RelationField>),
    CheckedUpdateManyInput(Model),
    UncheckedUpdateManyInput(Model, Option<RelationField>),
    CheckedUpdateOneInput(Model, Option<RelationField>),
    CompositeCreateEnvelopeInput(CompositeType, FieldArity),
    CompositeCreateInput(CompositeType),
    CompositeDeleteManyInput(CompositeType),
    CompositeUpdateEnvelopeInput(CompositeType, FieldArity),
    CompositeUpdateInput(CompositeType),
    CompositeUpdateManyInput(CompositeType),
    CompositeUpsertObjectInput(CompositeType),
    CreateManyInput(Model, Option<RelationField>),
    CreateManyAndReturnOutput(Model),
    CreateOneScalarList(ScalarField),
    Enum(InternalEnum),
    FieldUpdateOperationsInput(bool, String),
    Model(Model),
    NestedUpsertManyInput(RelationField),
    NestedUpsertOneInput(RelationField),
    OrderByAggregateInput(ParentContainer, String),
    OrderByInput(ParentContainer, String),
    OrderByRelevanceFieldEnum(ParentContainer),
    OrderByRelevanceInput(ParentContainer),
    OrderByToManyAggregateInput(ParentContainer),
    RelationCreateInput(RelationField, RelationField, bool),
    RelationLoadStrategy,
    RelationUpdateInput(RelationField, RelationField, bool),
    ScalarFieldEnum(Model),
    ScalarFilterInput(Model, bool),
    ScalarListFilterInput(Zipper<TypeIdentifier>, bool),
    ScalarListUpdateInput(ScalarField),
    ToManyCompositeFilterInput(CompositeType),
    ToManyRelationFilterInput(Model),
    ToOneCompositeFilterInput(CompositeType, FieldArity),
    ToOneRelationFilterInput(Model, FieldArity),
    TransactionIsolationLevel,
    UncheckedCreateInput(Model, Option<RelationField>),
    UncheckedUpdateOneInput(Model, Option<RelationField>),
    UpdateManyWhereCombinationInput(RelationField),
    UpdateOneWhereCombinationInput(RelationField),
    UpdateToOneRelWhereCombinationInput(RelationField),
    WhereInput(ParentContainer),
    WhereUniqueInput(Model),
    Raw(String),
}

impl std::fmt::Display for IdentifierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentifierType::Raw(s) => f.write_str(s),
            IdentifierType::AffectedRowsOutput => f.write_str("AffectedRowsOutput"),
            IdentifierType::SortOrder => f.write_str(ordering::SORT_ORDER),
            IdentifierType::Query => f.write_str("Query"),
            IdentifierType::Mutation => f.write_str("Mutation"),
            IdentifierType::Model(model) => f.write_str(model.name()),
            IdentifierType::Enum(enum_id) => f.write_str(enum_id.name()),
            IdentifierType::ScalarFieldEnum(model) => {
                write!(f, "{}ScalarFieldEnum", capitalize(model.name()))
            }
            IdentifierType::OrderByRelevanceFieldEnum(container) => {
                write!(f, "{}OrderByRelevanceFieldEnum", container.name())
            }
            IdentifierType::TransactionIsolationLevel => f.write_str("TransactionIsolationLevel"),
            IdentifierType::OrderByInput(container, suffix) => {
                write!(f, "{}OrderBy{}Input", container.name(), suffix)
            }
            IdentifierType::OrderByAggregateInput(container, suffix) => {
                write!(f, "{}{}OrderByAggregateInput", container.name(), suffix)
            }
            IdentifierType::OrderByToManyAggregateInput(container) => {
                let container_type = match container {
                    ParentContainer::Model(_) => "Relation",
                    ParentContainer::CompositeType(_) => "Composite",
                };

                write!(f, "{}OrderBy{}AggregateInput", container.name(), container_type)
            }
            IdentifierType::OrderByRelevanceInput(container) => {
                write!(f, "{}OrderByRelevanceInput", container.name())
            }
            IdentifierType::CreateOneScalarList(sf) => {
                write!(f, "{}Create{}Input", sf.container().name(), sf.name())
            }
            IdentifierType::FieldUpdateOperationsInput(nullable, prefix) => {
                // Different names are required to construct and cache different objects.
                // - "Nullable" affects the `set` operation (`set` is nullable)
                let nullable = if *nullable { "Nullable" } else { "" };

                write!(f, "{nullable}{prefix}FieldUpdateOperationsInput")
            }
            IdentifierType::RelationCreateInput(parent_field, related_field, unchecked) => {
                let related_model = related_field.model();

                let arity_part = if parent_field.is_list() {
                    "NestedMany"
                } else {
                    "NestedOne"
                };
                let unchecked_part = if *unchecked { "Unchecked" } else { "" };

                write!(
                    f,
                    "{}{}Create{}Without{}Input",
                    related_model.name(),
                    unchecked_part,
                    arity_part,
                    capitalize(related_field.name())
                )
            }
            IdentifierType::CompositeCreateEnvelopeInput(ct, field_arity) => {
                let arity = if field_arity.is_optional() {
                    "Nullable"
                } else if field_arity.is_list() {
                    "List"
                } else {
                    ""
                };

                write!(f, "{}{}CreateEnvelopeInput", ct.name(), arity)
            }
            IdentifierType::CompositeCreateInput(ct) => {
                write!(f, "{}CreateInput", ct.name())
            }
            IdentifierType::ScalarListUpdateInput(sf) => {
                write!(f, "{}Update{}Input", sf.container().name(), sf.name())
            }
            IdentifierType::RelationUpdateInput(parent_field, related_field, unchecked) => {
                let related_model = related_field.model();

                // Compute input object name
                let arity_part = match (parent_field.is_list(), parent_field.is_required()) {
                    (true, _) => "Many",
                    (false, true) => "OneRequired",
                    (false, false) => "One",
                };

                let unchecked_part = if *unchecked { "Unchecked" } else { "" };

                write!(
                    f,
                    "{}{}Update{}Without{}NestedInput",
                    related_model.name(),
                    unchecked_part,
                    arity_part,
                    capitalize(related_field.name())
                )
            }
            IdentifierType::CompositeUpdateEnvelopeInput(ct, field_arity) => {
                let arity = if field_arity.is_optional() {
                    "Nullable"
                } else if field_arity.is_list() {
                    "List"
                } else {
                    ""
                };

                write!(f, "{}{}UpdateEnvelopeInput", ct.name(), arity)
            }
            IdentifierType::CompositeUpdateInput(ct) => {
                write!(f, "{}UpdateInput", ct.name())
            }
            IdentifierType::CompositeUpsertObjectInput(ct) => {
                write!(f, "{}UpsertInput", ct.name())
            }
            IdentifierType::CompositeUpdateManyInput(ct) => {
                write!(f, "{}UpdateManyInput", ct.name())
            }
            IdentifierType::CompositeDeleteManyInput(ct) => {
                write!(f, "{}DeleteManyInput", ct.name())
            }
            IdentifierType::ToManyRelationFilterInput(related_model) => {
                write!(f, "{}ListRelationFilter", capitalize(related_model.name()))
            }
            IdentifierType::ToOneRelationFilterInput(related_model, arity) => {
                let nullable = if arity.is_optional() { "Nullable" } else { "" };

                write!(f, "{}{}RelationFilter", capitalize(related_model.name()), nullable)
            }
            IdentifierType::ToOneCompositeFilterInput(ct, arity) => {
                let nullable = if arity.is_optional() { "Nullable" } else { "" };

                write!(f, "{}{}CompositeFilter", capitalize(ct.name()), nullable)
            }
            IdentifierType::ToManyCompositeFilterInput(ct) => {
                write!(f, "{}CompositeListFilter", capitalize(ct.name()))
            }
            IdentifierType::ScalarListFilterInput(type_identifier, required) => f.write_str(&scalar_filter_name(
                &type_identifier.id.type_name(&type_identifier.dm.schema),
                true,
                !required,
                false,
                false,
            )),
            IdentifierType::ScalarFilterInput(model, includes_aggregate) => {
                let aggregate = if *includes_aggregate { "WithAggregates" } else { "" };

                write!(f, "{}ScalarWhere{}Input", model.name(), aggregate)
            }
            IdentifierType::WhereInput(container) => {
                write!(f, "{}WhereInput", container.name())
            }
            IdentifierType::WhereUniqueInput(model) => {
                write!(f, "{}WhereUniqueInput", model.name())
            }
            IdentifierType::CheckedUpdateOneInput(model, related_field) => match related_field {
                Some(field) => write!(f, "{}UpdateWithout{}Input", model.name(), capitalize(field.name())),
                _ => write!(f, "{}UpdateInput", model.name()),
            },
            IdentifierType::UncheckedUpdateOneInput(model, related_field) => match related_field {
                Some(field) => write!(
                    f,
                    "{}UncheckedUpdateWithout{}Input",
                    model.name(),
                    capitalize(field.name())
                ),
                _ => write!(f, "{}UncheckedUpdateInput", model.name()),
            },
            IdentifierType::UpdateOneWhereCombinationInput(related_field) => {
                let related_model = related_field.model();

                write!(
                    f,
                    "{}UpdateWithWhereUniqueWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::UpdateToOneRelWhereCombinationInput(related_field) => {
                let related_model = related_field.model();

                write!(
                    f,
                    "{}UpdateToOneWithWhereWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::CheckedUpdateManyInput(model) => {
                write!(f, "{}UpdateManyMutationInput", model.name())
            }
            IdentifierType::UpdateManyWhereCombinationInput(related_field) => {
                let related_model = related_field.model();

                write!(
                    f,
                    "{}UpdateManyWithWhereWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::NestedUpsertManyInput(related_field) => {
                let related_model = related_field.model();

                write!(
                    f,
                    "{}UpsertWithWhereUniqueWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::NestedUpsertOneInput(related_field) => {
                let related_model = related_field.model();

                write!(
                    f,
                    "{}UpsertWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::CheckedCreateInput(model, related_field) => match related_field {
                Some(ref rf) => write!(f, "{}CreateWithout{}Input", model.name(), capitalize(rf.name())),
                _ => write!(f, "{}CreateInput", model.name()),
            },
            IdentifierType::UncheckedCreateInput(model, related_field) => match related_field {
                Some(ref rf) => write!(
                    f,
                    "{}UncheckedCreateWithout{}Input",
                    model.name(),
                    capitalize(rf.name())
                ),
                _ => write!(f, "{}UncheckedCreateInput", model.name()),
            },
            IdentifierType::CreateManyInput(model, related_field) => match related_field {
                Some(ref rf) => write!(f, "{}CreateMany{}Input", model.name(), capitalize(rf.name())),
                _ => write!(f, "{}CreateManyInput", model.name()),
            },
            IdentifierType::CreateManyAndReturnOutput(model) => {
                write!(f, "CreateMany{}AndReturnOutputType", model.name())
            }
            IdentifierType::UncheckedUpdateManyInput(model, related_field) => match related_field {
                Some(rf) => write!(
                    f,
                    "{}UncheckedUpdateManyWithout{}Input",
                    model.name(),
                    capitalize(rf.name())
                ),
                _ => write!(f, "{}UncheckedUpdateManyInput", model.name()),
            },
            IdentifierType::RelationLoadStrategy => write!(f, "RelationLoadStrategy"),
        }
    }
}

impl From<String> for IdentifierType {
    fn from(value: String) -> Self {
        Self::Raw(value)
    }
}

impl From<&str> for IdentifierType {
    fn from(value: &str) -> Self {
        Self::Raw(value.to_owned())
    }
}
