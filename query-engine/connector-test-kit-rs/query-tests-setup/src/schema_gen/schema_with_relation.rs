use crate::{constants::*, query_params::*, references::*, relation_field::*};

#[derive(Debug, Clone)]
pub struct DatamodelWithParams {
    pub datamodel: String,
    pub parent: QueryParams,
    pub child: QueryParams,
}

pub fn schema_with_relation(
    on_parent: RelationField,
    on_child: RelationField,
    without_params: bool,
) -> Vec<DatamodelWithParams> {
    let is_required_1to1 = on_parent.is_required() && on_child.is_required();

    if !is_required_1to1 {
        panic!("required 1:1 relations must be rejected by the parser already");
    }

    // Query Params
    let id_param = QueryParams::new("id", QueryParamsWhere::identifier("id"), QueryParamsMulti::multi("id"));
    let compound_id_param = {
        let fields = vec!["id_1", "id_2"];
        let arg_name = "id_1_id_2";
        QueryParams::new(
            "id_1, id_2",
            QueryParamsWhere::compound_identifier(fields.clone(), arg_name),
            QueryParamsMulti::multi_compound(fields, arg_name),
        )
    };
    let parent_unique_params = vec![
        QueryParams::new("p", QueryParamsWhere::identifier("p"), QueryParamsMulti::multi("p")),
        {
            let fields = vec!["p_1", "p_2"];
            let arg_name = "p_1_p_2";
            QueryParams::new(
                "p_1, p_2",
                QueryParamsWhere::compound_identifier(fields.clone(), arg_name),
                QueryParamsMulti::multi_compound(fields, arg_name),
            )
        },
    ];
    let child_unique_params = vec![
        QueryParams::new("c", QueryParamsWhere::identifier("c"), QueryParamsMulti::multi("c")),
        {
            let fields = vec!["c_1", "c_2"];
            let arg_name = "c_1_c_2";
            QueryParams::new(
                "c_1, c_2",
                QueryParamsWhere::compound_identifier(fields.clone(), arg_name),
                QueryParamsMulti::multi_compound(fields, arg_name),
            )
        },
    ];

    // we only support singular id fields with implicit many to many relations. https://github.com/prisma/prisma/issues/2262
    let id_options = if on_parent.is_list() && on_child.is_list() {
        SIMPLE_ID_OPTIONS.to_vec()
    } else {
        FULL_ID_OPTIONS.to_vec()
    };

    // TODO: How to configure simple mode??
    let simple = true;
    let mut datamodels: Vec<DatamodelWithParams> = vec![];

    for parent_id in id_options.iter() {
        for child_id in id_options.iter() {
            for child_reference_to_parent in child_references(simple, parent_id, &on_parent, &on_child) {
                for parent_reference_to_child in
                    parent_references(simple, child_id, &child_reference_to_parent, &on_parent, &on_child)
                {
                    let is_virtual_req_rel_field = on_parent.is_required()
                        && parent_reference_to_child.render() == RelationReference::NoRef.render();

                    if !is_virtual_req_rel_field {
                        continue;
                    }

                    let parent_params = if without_params {
                        vec![id_param.clone()]
                    } else {
                        match *parent_id {
                            SIMPLE_ID => {
                                let mut params = parent_unique_params.clone();
                                params.push(id_param.clone());

                                params
                            }
                            COMPOUND_ID => {
                                let mut params = parent_unique_params.clone();
                                params.push(compound_id_param.clone());

                                params
                            }
                            NO_ID => parent_unique_params.clone(),
                            _ => unimplemented!(),
                        }
                    };

                    let child_params = if without_params {
                        vec![id_param.clone()]
                    } else {
                        match *child_id {
                            SIMPLE_ID => {
                                let mut params = child_unique_params.clone();
                                params.push(id_param.clone());

                                params
                            }
                            COMPOUND_ID => {
                                let mut params = child_unique_params.clone();
                                params.push(compound_id_param.clone());

                                params
                            }
                            NO_ID => child_unique_params.clone(),
                            _ => unimplemented!(),
                        }
                    };

                    for parent_param in parent_params.iter() {
                        for child_param in child_params.iter() {
                            let datamodel = indoc::formatdoc! {"
                                model Parent {{
                                    p             String    @unique
                                    p_1           String
                                    p_2           String
                                    {parent_field}         {parent_reference_to_child}
                                    non_unique    String?
                                    {parent_id}
                
                                    @@unique([p_1, p_2])
                                }}
                
                                model Child {{
                                    c              String    @unique
                                    c_1         .  String
                                    c_2            String
                                    {child_field}          {child_reference_to_parent}
                                    non_unique     String?
                                    {child_id}
                
                                    @@unique([c_1, c_2])
                                }}
                                ",
                                parent_field = on_parent.field(),
                                parent_reference_to_child = parent_reference_to_child.render(),
                                parent_id = parent_id,
                                child_field = on_child.field(),
                                child_reference_to_parent = child_reference_to_parent.render(),
                                child_id = child_id
                            };

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

    datamodels
}
