use crate::{capitalize, scalar_filter_name};

use prisma_models::{prelude::*, *};

/// Enum used to represent unique schema type names.
/// It helps deferring the allocation + formatting of strings
/// during the initialization of the schema, which proved to be very costly on large schemas.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum IdentifierType {
    CheckedCreateInput(Model, Option<RelationField>),
    CheckedUpdateManyInput(Model),
    CheckedUpdateOneInput(Model, Option<RelationField>),
    CompositeCreateEnvelopeInput(CompositeField),
    CompositeCreateInput(CompositeField),
    CompositeDeleteManyInput(CompositeField),
    CompositeUpdateEnvelopeInput(CompositeField),
    CompositeUpdateInput(CompositeField),
    CompositeUpdateManyInput(CompositeField),
    CompositeUpsertObjectInput(CompositeField),
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
    RelationCreateInput(RelationField, bool),
    RelationUpdateInput(RelationField, bool),
    ScalarFieldEnum(Model),
    ScalarFilterInput(Model, bool),
    ScalarListFilterInput(ScalarField),
    ScalarListUpdateInput(ScalarField),
    ToManyCompositeFilterInput(CompositeField),
    ToManyRelationFilterInput(RelationField),
    ToOneCompositeFilterInput(CompositeField),
    ToOneRelationFilterInput(RelationField),
    TransactionIsolationLevel,
    UncheckedCreateInput(Model, Option<RelationField>),
    UncheckedUpdateManyInput(Model, Option<RelationField>),
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
            IdentifierType::RelationCreateInput(rf, unchecked) => {
                let related_model = rf.related_model();
                let related_field = rf.related_field();

                let arity_part = if rf.is_list() { "NestedMany" } else { "NestedOne" };
                let unchecked_part = if *unchecked { "Unchecked" } else { "" };

                format!(
                    "{}{}Create{}Without{}Input",
                    related_model.name(),
                    unchecked_part,
                    arity_part,
                    capitalize(related_field.name())
                )
            }
            IdentifierType::CompositeCreateEnvelopeInput(cf) => {
                let arity = if cf.is_optional() {
                    "Nullable"
                } else if cf.is_list() {
                    "List"
                } else {
                    ""
                };

                format!("{}{}CreateEnvelopeInput", cf.typ().name(), arity)
            }
            IdentifierType::CompositeCreateInput(cf) => {
                format!("{}CreateInput", cf.typ().name())
            }
            IdentifierType::ScalarListUpdateInput(sf) => {
                format!("{}Update{}Input", sf.container().name(), sf.name())
            }
            IdentifierType::RelationUpdateInput(rf, unchecked) => {
                let related_model = rf.related_model();
                let related_field = rf.related_field();

                // Compute input object name
                let arity_part = match (rf.is_list(), rf.is_required()) {
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
            IdentifierType::CompositeUpdateEnvelopeInput(cf) => {
                let arity = if cf.is_optional() {
                    "Nullable"
                } else if cf.is_list() {
                    "List"
                } else {
                    ""
                };

                format!("{}{}UpdateEnvelopeInput", cf.typ().name(), arity)
            }
            IdentifierType::CompositeUpdateInput(cf) => {
                format!("{}UpdateInput", cf.typ().name())
            }
            IdentifierType::CompositeUpsertObjectInput(cf) => {
                format!("{}UpsertInput", cf.typ().name())
            }
            IdentifierType::CompositeUpdateManyInput(cf) => {
                format!("{}UpdateManyInput", cf.typ().name())
            }
            IdentifierType::CompositeDeleteManyInput(cf) => {
                format!("{}DeleteManyInput", cf.typ().name())
            }
            IdentifierType::ToManyRelationFilterInput(rf) => {
                let related_model = rf.related_model();

                format!("{}ListRelationFilter", capitalize(related_model.name()))
            }
            IdentifierType::ToOneRelationFilterInput(rf) => {
                let related_model = rf.related_model();

                format!("{}RelationFilter", capitalize(related_model.name()))
            }
            IdentifierType::ToOneCompositeFilterInput(cf) => {
                let nullable = if cf.is_optional() { "Nullable" } else { "" };

                format!("{}{}CompositeFilter", capitalize(cf.typ().name()), nullable)
            }
            IdentifierType::ToManyCompositeFilterInput(cf) => {
                format!("{}CompositeListFilter", capitalize(cf.typ().name()))
            }
            IdentifierType::ScalarListFilterInput(sf) => scalar_filter_name(
                &sf.type_identifier().type_name(&sf.dm.schema),
                true,
                !sf.is_required(),
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
            IdentifierType::CheckedUpdateOneInput(model, parent_field) => {
                match parent_field.as_ref().map(|pf| pf.related_field()) {
                    Some(ref f) => format!("{}UpdateWithout{}Input", model.name(), capitalize(f.name())),
                    _ => format!("{}UpdateInput", model.name()),
                }
            }
            IdentifierType::UncheckedUpdateOneInput(model, parent_field) => {
                match parent_field.as_ref().map(|pf| pf.related_field()) {
                    Some(ref f) => format!("{}UncheckedUpdateWithout{}Input", model.name(), capitalize(f.name())),
                    _ => format!("{}UncheckedUpdateInput", model.name()),
                }
            }
            IdentifierType::UpdateOneWhereCombinationInput(parent_field) => {
                let related_model = parent_field.related_model();

                format!(
                    "{}UpdateWithWhereUniqueWithout{}Input",
                    related_model.name(),
                    capitalize(parent_field.related_field().name())
                )
            }
            IdentifierType::UpdateToOneRelWhereCombinationInput(parent_field) => {
                let related_model = parent_field.related_model();

                format!(
                    "{}UpdateToOneWithWhereWithout{}Input",
                    related_model.name(),
                    capitalize(parent_field.related_field().name())
                )
            }
            IdentifierType::CheckedUpdateManyInput(model) => {
                format!("{}UpdateManyMutationInput", model.name())
            }
            IdentifierType::UncheckedUpdateManyInput(model, parent_field) => {
                match parent_field.as_ref().map(|pf| pf.related_field()) {
                    Some(ref f) => format!(
                        "{}UncheckedUpdateManyWithout{}Input",
                        model.name(),
                        capitalize(f.related_field().name())
                    ),
                    _ => format!("{}UncheckedUpdateManyInput", model.name()),
                }
            }
            IdentifierType::UpdateManyWhereCombinationInput(parent_field) => {
                let related_model = parent_field.related_model();

                format!(
                    "{}UpdateManyWithWhereWithout{}Input",
                    related_model.name(),
                    capitalize(parent_field.related_field().name())
                )
            }
            IdentifierType::NestedUpsertManyInput(parent_field) => {
                let related_model = parent_field.related_model();

                format!(
                    "{}UpsertWithWhereUniqueWithout{}Input",
                    related_model.name(),
                    capitalize(parent_field.related_field().name())
                )
            }
            IdentifierType::NestedUpsertOneInput(parent_field) => {
                let related_model = parent_field.related_model();

                format!(
                    "{}UpsertWithout{}Input",
                    related_model.name(),
                    capitalize(parent_field.related_field().name())
                )
            }
            IdentifierType::CheckedCreateInput(model, parent_field) => {
                match parent_field.as_ref().map(|pf| pf.related_field()) {
                    Some(ref f) => format!("{}CreateWithout{}Input", model.name(), capitalize(f.name())),
                    _ => format!("{}CreateInput", model.name()),
                }
            }
            IdentifierType::UncheckedCreateInput(model, parent_field) => {
                match parent_field.as_ref().map(|pf| pf.related_field()) {
                    Some(ref f) => format!("{}UncheckedCreateWithout{}Input", model.name(), capitalize(f.name())),
                    _ => format!("{}UncheckedCreateInput", model.name()),
                }
            }
            IdentifierType::CreateManyInput(model, parent_field) => {
                match parent_field.as_ref().map(|pf| pf.related_field()) {
                    Some(ref f) => format!("{}CreateMany{}Input", model.name(), capitalize(f.name())),
                    _ => format!("{}CreateManyInput", model.name()),
                }
            }
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
