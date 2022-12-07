use super::*;
use psl::datamodel_connector::ConnectorCapability;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatamodelWithParams {
    datamodel: String,
    parent: QueryParams,
    child: QueryParams,
}

impl DatamodelWithParams {
    /// Get a reference to the datamodel with params's datamodel.
    pub fn datamodel(&self) -> &str {
        self.datamodel.as_str()
    }

    /// Get a reference to the datamodel with params's parent.
    pub fn parent(&self) -> &QueryParams {
        &self.parent
    }

    /// Get a reference to the datamodel with params's child.
    pub fn child(&self) -> &QueryParams {
        &self.child
    }
}

impl FromStr for DatamodelWithParams {
    type Err = serde_json::Error;

    fn from_str(from: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(from)
    }
}

impl TryFrom<DatamodelWithParams> for String {
    type Error = serde_json::Error;

    fn try_from(from: DatamodelWithParams) -> Result<Self, Self::Error> {
        serde_json::to_string(&from)
    }
}

pub type DatamodelsAndCapabilities = (Vec<DatamodelWithParams>, Vec<Vec<ConnectorCapability>>);

pub(crate) fn schema_with_relation(
    on_parent: &RelationField,
    on_child: &RelationField,
    id_only: bool,
) -> DatamodelsAndCapabilities {
    let is_required_1to1 = on_parent.is_required() && on_child.is_required();

    if is_required_1to1 {
        panic!("required 1:1 relations must be rejected by the parser already");
    }

    // Query Params
    let id_param = QueryParams::new(
        "id",
        QueryParamsWhere::identifier("id"),
        QueryParamsWhereMany::many_ids("id"),
    );

    let compound_id_param = {
        let fields = vec!["id_1", "id_2"];
        let arg_name = "id_1_id_2";

        QueryParams::new(
            "id_1, id_2",
            QueryParamsWhere::compound_identifier(fields.clone(), arg_name),
            QueryParamsWhereMany::many_compounds(fields, arg_name),
        )
    };

    let parent_unique_params = vec![
        QueryParams::new(
            "p",
            QueryParamsWhere::identifier("p"),
            QueryParamsWhereMany::many_ids("p"),
        ),
        {
            let fields = vec!["p_1", "p_2"];
            let arg_name = "p_1_p_2";

            QueryParams::new(
                "p_1, p_2",
                QueryParamsWhere::compound_identifier(fields.clone(), arg_name),
                QueryParamsWhereMany::many_compounds(fields, arg_name),
            )
        },
    ];

    let child_unique_params = vec![
        QueryParams::new(
            "c",
            QueryParamsWhere::identifier("c"),
            QueryParamsWhereMany::many_ids("c"),
        ),
        {
            let fields = vec!["c_1", "c_2"];
            let arg_name = "c_1_c_2";

            QueryParams::new(
                "c_1, c_2",
                QueryParamsWhere::compound_identifier(fields.clone(), arg_name),
                QueryParamsWhereMany::many_compounds(fields, arg_name),
            )
        },
    ];

    // we only support singular id fields with implicit many to many relations. https://github.com/prisma/prisma/issues/2262
    let id_options = if on_parent.is_list() && on_child.is_list() {
        SIMPLE_ID_OPTIONS.to_vec()
    } else {
        FULL_ID_OPTIONS.to_vec()
    };

    // Reduces the amount of generated tests when `true`
    let simple_test_mode = std::env::var("SIMPLE_TEST_MODE").is_ok();
    let mut datamodels: Vec<DatamodelWithParams> = vec![];
    let mut required_capabilities: Vec<Vec<ConnectorCapability>> = vec![];

    for parent_id in id_options.iter() {
        for child_id in id_options.iter() {
            // Based on Id and relation fields
            for child_ref_to_parent in child_references(simple_test_mode, parent_id, on_parent, on_child) {
                for parent_ref_to_child in
                    parent_references(simple_test_mode, child_id, &child_ref_to_parent, on_parent, on_child)
                {
                    // TODO: The RelationReference.render() equality is a hack. Implement PartialEq instead
                    let is_virtual_req_rel_field =
                        on_parent.is_required() && parent_ref_to_child.render() == RelationReference::NoRef.render();

                    // skip required virtual relation fields as those are disallowed in a Prisma Schema
                    if is_virtual_req_rel_field {
                        continue;
                    }

                    // Only based on id
                    let parent_params = if id_only {
                        vec![id_param.clone()]
                    } else {
                        match *parent_id {
                            Identifier::Simple => parent_unique_params.clone_push(&id_param),
                            Identifier::Compound => parent_unique_params.clone_push(&compound_id_param),
                            Identifier::None => parent_unique_params.clone(),
                        }
                    };

                    let child_params = if id_only {
                        vec![id_param.clone()]
                    } else {
                        match *child_id {
                            Identifier::Simple => child_unique_params.clone_push(&id_param),
                            Identifier::Compound => child_unique_params.clone_push(&compound_id_param),
                            Identifier::None => child_unique_params.clone(),
                        }
                    };

                    for parent_param in parent_params.iter() {
                        for child_param in child_params.iter() {
                            let (parent_field, child_field) =
                                render_relation_fields(on_parent, &parent_ref_to_child, on_child, &child_ref_to_parent);

                            let datamodel = indoc::formatdoc! {"
                                model Parent {{
                                    p             String    @unique
                                    p_1           String
                                    p_2           String
                                    {parent_field}
                                    non_unique    String?
                                    {parent_id}

                                    @@unique([p_1, p_2])
                                }}

                                model Child {{
                                    c              String    @unique
                                    c_1            String
                                    c_2            String
                                    {child_field}
                                    non_unique     String?
                                    {child_id}

                                    @@unique([c_1, c_2])
                                }}
                            "};

                            let mut required_capabilities_for_dm = vec![];

                            match (parent_id, child_id) {
                                (Identifier::Compound, _) | (_, Identifier::Compound) => {
                                    required_capabilities_for_dm.push(ConnectorCapability::CompoundIds)
                                }
                                (Identifier::None, _) | (_, Identifier::None) => {
                                    required_capabilities_for_dm.push(ConnectorCapability::AnyId)
                                }
                                _ => (),
                            }

                            required_capabilities.push(required_capabilities_for_dm);

                            datamodels.push(DatamodelWithParams {
                                datamodel,
                                parent: parent_param.clone(),
                                child: child_param.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    (datamodels, required_capabilities)
}

fn render_relation_fields(
    parent: &RelationField,
    parent_ref_to_child: &RelationReference,
    child: &RelationField,
    child_ref_to_parent: &RelationReference,
) -> (String, String) {
    if parent.is_list() && child.is_list() {
        let rendered_parent = format!("#m2m({}, {}, id, String)", parent.field_name(), parent.type_name());
        let rendered_child = format!("#m2m({}, {}, id, String)", child.field_name(), child.type_name(),);

        (rendered_parent, rendered_child)
    } else {
        let mut rendered_parent = format!(
            "{} {} {}",
            parent.field_name(),
            parent.type_name(),
            parent_ref_to_child.render()
        );

        let mut rendered_child = format!(
            "{} {} {}",
            child.field_name(),
            child.type_name(),
            child_ref_to_parent.render()
        );

        if !child.is_list() && !parent.is_list() {
            let child_unique = match child_ref_to_parent {
                RelationReference::SimpleChildId(_) => r#"@@unique([childId])"#,
                RelationReference::SimpleParentId(_) => r#"@@unique([parentId])"#,
                RelationReference::CompoundParentId(_) => r#"@@unique([parent_id_1, parent_id_2])"#,
                RelationReference::CompoundChildId(_) => r#"@@unique([child_id_1, child_id_2])"#,
                RelationReference::ParentReference(_) => r#"@@unique([parentRef])"#,
                RelationReference::CompoundParentReference(_) => r#"@@unique([parent_p_1, parent_p_2])"#,
                RelationReference::ChildReference(_) => r#"@@unique([parent_c])"#,
                RelationReference::CompoundChildReference(_) => r#"@@unique([child_c_1, child_c_2])"#,
                RelationReference::IdReference => "",
                RelationReference::NoRef => "",
            };

            let parent_unique = match parent_ref_to_child {
                RelationReference::SimpleChildId(_) => r#"@@unique([childId])"#,
                RelationReference::SimpleParentId(_) => r#"@@unique([parentId])"#,
                RelationReference::CompoundParentId(_) => r#"@@unique([parent_id_1, parent_id_2])"#,
                RelationReference::CompoundChildId(_) => r#"@@unique([child_id_1, child_id_2])"#,
                RelationReference::ParentReference(_) => r#"@@unique([parentRef])"#,
                RelationReference::CompoundParentReference(_) => r#"@@unique([parent_p_1, parent_p_2])"#,
                RelationReference::ChildReference(_) => r#"@@unique([parent_c])"#,
                RelationReference::CompoundChildReference(_) => r#"@@unique([child_c_1, child_c_2])"#,
                RelationReference::IdReference => "",
                RelationReference::NoRef => "",
            };

            rendered_child.push('\n');
            rendered_child.push_str(child_unique);

            rendered_parent.push('\n');
            rendered_parent.push_str(parent_unique);
        }

        (rendered_parent, rendered_child)
    }
}
