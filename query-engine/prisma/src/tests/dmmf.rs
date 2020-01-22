use query_core::{BuildMode, QuerySchema, QuerySchemaBuilder, SupportedCapabilities};
use std::sync::Arc;

fn get_query_schema(datamodel_string: &str) -> (QuerySchema, datamodel::dml::Datamodel) {
    let lifted_datamodel = datamodel::parse_datamodel(datamodel_string).unwrap();
    let internal_datamodel = prisma_models::DatamodelConverter::convert(&lifted_datamodel).build("blah".to_owned());
    let supported_capabilities = SupportedCapabilities::empty();
    (
        QuerySchemaBuilder::new(&internal_datamodel, &supported_capabilities, BuildMode::Modern).build(),
        lifted_datamodel,
    )
}

#[test]
fn dmmf_create_inputs_without_fields_for_parent_records_are_correct() {
    let dm = r#"
        model Blog {
            blogId String @id
            postsField Post[]
        }

        model Post {
            postId String @id
            blogField Blog?
            tagsField Tag[]
        }

        model Tag {
            tagId String @id
            postsField Post[]
        }
    "#;

    let (query_schema, datamodel) = get_query_schema(dm);

    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));

    let inputs = &dmmf.schema.input_types;

    let create_post_from_blog = inputs
        .iter()
        .find(|input| input.name == "PostCreateWithoutBlogFieldInput")
        .expect("finding PostCreateWithoutBlogFieldInput");

    let create_post_from_blog_fields: Vec<(&str, &str)> = create_post_from_blog
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.input_type.typ.as_str()))
        .collect();

    assert_eq!(
        create_post_from_blog_fields,
        &[
            ("postId", "String"),
            ("tagsField", "TagCreateManyWithoutPostsFieldInput")
        ]
    );

    let create_post_from_tags = inputs
        .iter()
        .find(|input| input.name == "PostCreateWithoutTagsFieldInput")
        .expect("finding PostCreateWithoutTagsFieldInput");

    let create_post_from_tags_fields: Vec<(&str, &str)> = create_post_from_tags
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.input_type.typ.as_str()))
        .collect();

    assert_eq!(
        create_post_from_tags_fields,
        &[
            ("postId", "String"),
            ("blogField", "BlogCreateOneWithoutPostsFieldInput")
        ]
    );
}
