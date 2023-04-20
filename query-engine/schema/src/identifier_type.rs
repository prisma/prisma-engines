use crate::{capitalize, scalar_filter_name};
use prisma_models::{ast::FieldArity, prelude::*, *};

/// Enum used to represent unique schema type names.
/// It helps deferring the allocation + formatting of strings
/// during the initialization of the schema, which proved to be very costly on large schemas.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum IdentifierType {
    CheckedCreateInput(Model, Option<RelationField>),
    CheckedUpdateManyInput(Model),
    CheckedUpdateOneInput(Model, Option<RelationField>),
    CompositeCreateEnvelopeInput(CompositeType, FieldArity),
    CompositeCreateInput(CompositeType),
    CompositeDeleteManyInput(CompositeType),
    CompositeUpdateEnvelopeInput(CompositeType, FieldArity),
    CompositeUpdateInput(CompositeType),
    CompositeUpdateManyInput(CompositeType),
    CompositeUpsertObjectInput(CompositeType),
    CreateManyInput(Model, Option<RelationField>),
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
    RelationUpdateInput(RelationField, RelationField, bool),
    ScalarFieldEnum(Model),
    ScalarFilterInput(Model, bool),
    ScalarListFilterInput(Zipper<TypeIdentifier>, bool),
    ScalarListUpdateInput(ScalarField),
    ToManyCompositeFilterInput(CompositeType),
    ToManyRelationFilterInput(Model),
    ToOneCompositeFilterInput(CompositeType, FieldArity),
    ToOneRelationFilterInput(Model),
    TransactionIsolationLevel,
    UncheckedCreateInput(Model, Option<RelationField>),
    UncheckedUpdateOneInput(Model, Option<RelationField>),
    UpdateManyWhereCombinationInput(RelationField),
    UpdateOneWhereCombinationInput(RelationField),
    UpdateToOneRelWhereCombinationInput(RelationField),
    WhereInput(ParentContainer),
    WhereUniqueInput(Model),
    // Raw string identifier. Used when deferring the allocation + formatting is not worth it.
    Raw(String),
}

impl std::fmt::Display for IdentifierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            IdentifierType::Model(model) => model.name().to_string(),
            IdentifierType::Enum(enum_id) => enum_id.name().to_owned(),
            IdentifierType::ScalarFieldEnum(model) => {
                format!("{}ScalarFieldEnum", capitalize(model.name()))
            }
            IdentifierType::OrderByRelevanceFieldEnum(container) => {
                format!("{}OrderByRelevanceFieldEnum", container.name())
            }
            IdentifierType::TransactionIsolationLevel => "TransactionIsolationLevel".to_string(),
            IdentifierType::OrderByInput(container, suffix) => {
                format!("{}OrderBy{}Input", container.name(), suffix)
            }
            IdentifierType::OrderByAggregateInput(container, suffix) => {
                format!("{}{}OrderByAggregateInput", container.name(), suffix)
            }
            IdentifierType::OrderByToManyAggregateInput(container) => {
                let container_type = match container {
                    ParentContainer::Model(_) => "Relation",
                    ParentContainer::CompositeType(_) => "Composite",
                };

                format!("{}OrderBy{}AggregateInput", container.name(), container_type)
            }
            IdentifierType::OrderByRelevanceInput(container) => {
                format!("{}OrderByRelevanceInput", container.name())
            }
            IdentifierType::CreateOneScalarList(sf) => {
                format!("{}Create{}Input", sf.container().name(), sf.name())
            }
            IdentifierType::FieldUpdateOperationsInput(nullable, prefix) => {
                // Different names are required to construct and cache different objects.
                // - "Nullable" affects the `set` operation (`set` is nullable)
                let nullable = if *nullable { "Nullable" } else { "" };

                format!("{nullable}{prefix}FieldUpdateOperationsInput")
            }
            IdentifierType::RelationCreateInput(parent_field, related_field, unchecked) => {
                let related_model = related_field.model();

                let arity_part = if parent_field.is_list() {
                    "NestedMany"
                } else {
                    "NestedOne"
                };
                let unchecked_part = if *unchecked { "Unchecked" } else { "" };

                format!(
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

                format!("{}{}CreateEnvelopeInput", ct.name(), arity)
            }
            IdentifierType::CompositeCreateInput(ct) => {
                format!("{}CreateInput", ct.name())
            }
            IdentifierType::ScalarListUpdateInput(sf) => {
                format!("{}Update{}Input", sf.container().name(), sf.name())
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

                format!(
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

                format!("{}{}UpdateEnvelopeInput", ct.name(), arity)
            }
            IdentifierType::CompositeUpdateInput(ct) => {
                format!("{}UpdateInput", ct.name())
            }
            IdentifierType::CompositeUpsertObjectInput(ct) => {
                format!("{}UpsertInput", ct.name())
            }
            IdentifierType::CompositeUpdateManyInput(ct) => {
                format!("{}UpdateManyInput", ct.name())
            }
            IdentifierType::CompositeDeleteManyInput(ct) => {
                format!("{}DeleteManyInput", ct.name())
            }
            IdentifierType::ToManyRelationFilterInput(related_model) => {
                format!("{}ListRelationFilter", capitalize(related_model.name()))
            }
            IdentifierType::ToOneRelationFilterInput(related_model) => {
                format!("{}RelationFilter", capitalize(related_model.name()))
            }
            IdentifierType::ToOneCompositeFilterInput(ct, arity) => {
                let nullable = if arity.is_optional() { "Nullable" } else { "" };

                format!("{}{}CompositeFilter", capitalize(ct.name()), nullable)
            }
            IdentifierType::ToManyCompositeFilterInput(ct) => {
                format!("{}CompositeListFilter", capitalize(ct.name()))
            }
            IdentifierType::ScalarListFilterInput(type_identifier, required) => scalar_filter_name(
                &type_identifier.id.type_name(&type_identifier.dm.schema),
                true,
                !required,
                false,
                false,
            ),
            IdentifierType::ScalarFilterInput(model, includes_aggregate) => {
                let aggregate = if *includes_aggregate { "WithAggregates" } else { "" };

                format!("{}ScalarWhere{}Input", model.name(), aggregate)
            }
            IdentifierType::WhereInput(container) => {
                format!("{}WhereInput", container.name())
            }
            IdentifierType::WhereUniqueInput(model) => {
                format!("{}WhereUniqueInput", model.name())
            }
            IdentifierType::CheckedUpdateOneInput(model, related_field) => match related_field {
                Some(f) => format!("{}UpdateWithout{}Input", model.name(), capitalize(f.name())),
                _ => format!("{}UpdateInput", model.name()),
            },
            IdentifierType::UncheckedUpdateOneInput(model, related_field) => match related_field {
                Some(f) => format!("{}UncheckedUpdateWithout{}Input", model.name(), capitalize(f.name())),
                _ => format!("{}UncheckedUpdateInput", model.name()),
            },
            IdentifierType::UpdateOneWhereCombinationInput(related_field) => {
                let related_model = related_field.model();

                format!(
                    "{}UpdateWithWhereUniqueWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::UpdateToOneRelWhereCombinationInput(related_field) => {
                let related_model = related_field.model();

                format!(
                    "{}UpdateToOneWithWhereWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::CheckedUpdateManyInput(model) => {
                format!("{}UpdateManyMutationInput", model.name())
            }
            IdentifierType::UpdateManyWhereCombinationInput(related_field) => {
                let related_model = related_field.model();

                format!(
                    "{}UpdateManyWithWhereWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::NestedUpsertManyInput(related_field) => {
                let related_model = related_field.model();

                format!(
                    "{}UpsertWithWhereUniqueWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::NestedUpsertOneInput(related_field) => {
                let related_model = related_field.model();

                format!(
                    "{}UpsertWithout{}Input",
                    related_model.name(),
                    capitalize(related_field.name())
                )
            }
            IdentifierType::CheckedCreateInput(model, related_field) => match related_field {
                Some(ref rf) => format!("{}CreateWithout{}Input", model.name(), capitalize(rf.name())),
                _ => format!("{}CreateInput", model.name()),
            },
            IdentifierType::UncheckedCreateInput(model, related_field) => match related_field {
                Some(ref rf) => format!("{}UncheckedCreateWithout{}Input", model.name(), capitalize(rf.name())),
                _ => format!("{}UncheckedCreateInput", model.name()),
            },
            IdentifierType::CreateManyInput(model, related_field) => match related_field {
                Some(ref rf) => format!("{}CreateMany{}Input", model.name(), capitalize(rf.name())),
                _ => format!("{}CreateManyInput", model.name()),
            },
            IdentifierType::Raw(name) => name.to_string(),
        };

        f.write_str(&name)
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
