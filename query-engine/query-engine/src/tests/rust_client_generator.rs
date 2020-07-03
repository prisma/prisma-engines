use super::dmmf::get_query_schema;
use crate::dmmf::*;
use codegen::{Scope, Struct, Type};
use serial_test::serial;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;

// Tests in this file run serially because the function `get_query_schema` depends on setting an env var.

#[test]
#[serial]
fn client_generator_test() {
    let dm = r#"
        model Blog {
            blogId String @id
            name   String
            posts  Post[]
        }

        model Post {
            postId      String  @id
            title       String
            subTitle    String
            subSubTitle String?
            blogId      String
            blog        Blog @relation(fields: blogId, references: blogId)
            
            @@unique([title, subTitle])
        }
    "#;

    let (query_schema, datamodel) = get_query_schema(dm);

    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let query_type = dmmf.schema.query_type();
    let mutation_type = dmmf.schema.mutation_type();

    dbg!(&dmmf);

    let mut scope = Scope::new();

    // ignore some warnings in the generated code for now while prototyping
    scope.raw("#![allow(non_snake_case, dead_code, unused_variables, non_camel_case_types)]");

    construct_input_types(&mut scope, &dmmf.schema.input_types);
    construct_output_types(&mut scope, &dmmf.schema.output_types);

    create_prisma_client_impl(&mut scope, &query_type);

    let mut client = codegen::Struct::new("PrismaClient");
    client.vis("pub");

    let mut fields_grouped_by_model: HashMap<String, Vec<&DMMFField>> = HashMap::new();
    for field in query_type.fields.iter().chain(mutation_type.fields.iter()) {
        if let Some(model) = field.model.as_ref() {
            let fields_vec = fields_grouped_by_model.entry(model.to_string()).or_insert(vec![]);
            fields_vec.push(&field)
        }
    }

    let model_impls = generate_operation_impls_for_models(&fields_grouped_by_model);
    for model_impl in model_impls.into_iter() {
        scope.push_struct(model_impl.the_struct);
        scope.push_impl(model_impl.the_impl);

        client.field(&format!("pub {}", &model_impl.model), model_impl.struct_name);
    }

    scope.push_struct(client);

    for an_enum in create_enums(&dmmf.schema.enums).into_iter() {
        scope.push_enum(an_enum);
    }

    write_to_disk(&scope.to_string());
}

fn generate_operation_impls_for_models(fields_grouped_by_model: &HashMap<String, Vec<&DMMFField>>) -> Vec<ModelImpl> {
    let mut ret = vec![];
    for (model, fields) in fields_grouped_by_model.iter() {
        let the_name = format!("{}Operations", &model);
        let mut the_struct = codegen::Struct::new(&the_name);
        the_struct.vis("pub");

        let mut the_impl = codegen::Impl::new(&the_name);
        for field in fields.iter() {
            let model_stripped_from_name = field.name.trim_end_matches(model);
            let function = the_impl.new_fn(&model_stripped_from_name).arg_ref_self();

            function.arg_ref_self();
            for arg in field.args.iter() {
                let arg_name = if &arg.name == "where" { "where_" } else { &arg.name };

                function.arg(arg_name, &map_type(&arg.input_type)).vis("pub");
            }
            function.ret(&map_type(&field.output_type));
            function.line("todo!()");
        }

        ret.push(ModelImpl {
            model: model.to_string(),
            struct_name: the_name.to_string(),
            the_struct,
            the_impl,
        });
    }

    ret
}

struct ModelImpl {
    model: String,
    struct_name: String,
    the_struct: codegen::Struct,
    the_impl: codegen::Impl,
}

fn construct_input_types(scope: &mut Scope, input_types: &Vec<DMMFInputType>) {
    scope.raw("/// INPUT TYPES");
    for input in input_types.iter() {
        if input.is_one_of {
            let input_enum = scope.new_enum(&input.name).vis("pub");
            for field in input.fields.iter() {
                input_enum
                    .new_variant(&field.name.pascal_case())
                    .tuple(&field.input_type.typ);
            }
        } else {
            let input_struct = new_pub_struct(scope, &input.name);
            for field in input.fields.iter() {
                if !field.input_type.typ.ends_with("WhereInput") {
                    // TODO: this needs boxing in the generated code.
                    input_struct.field(&format!("pub {}", &field.name()), map_type(&field.input_type));
                }
            }
            let impl_block = scope.new_impl(&input.name);
            let constructor = impl_block.new_fn("new").vis("pub").ret(&input.name).line("todo!()");
            for field in input.fields.iter() {
                if field.input_type.is_required {
                    constructor.arg(&field.name(), map_type(&field.input_type));
                }
            }

            for field in input.fields.iter() {
                if !field.input_type.is_required {
                    impl_block
                        .new_fn(&field.name())
                        .vis("pub")
                        .arg_self()
                        .arg(&field.name(), map_type(&field.input_type))
                        .ret(&input.name)
                        .line("todo!()");
                }
            }
        }
    }
}

fn construct_output_types(scope: &mut Scope, output_types: &Vec<DMMFOutputType>) {
    scope.raw("/// OUTPUT TYPES");

    let non_model_types = vec!["Query", "Mutation"];
    let model_output_types: Vec<_> = output_types
        .iter()
        .filter(|ot| !non_model_types.contains(&ot.name.as_str()))
        .collect();

    for output in model_output_types.iter() {
        let output_struct = new_pub_struct(scope, &output.name);
        for field in output.fields.iter() {
            output_struct.field(&format!("pub {}", &field.name), map_type(&field.output_type));
        }
    }
}

fn create_enums(enums: &Vec<DMMFEnum>) -> Vec<codegen::Enum> {
    let mut ret = vec![];
    for an_enum in enums.iter() {
        let mut the_enum = codegen::Enum::new(&an_enum.name);
        the_enum.vis("pub");
        for variant in an_enum.values.iter() {
            the_enum.new_variant(&variant);
        }
        ret.push(the_enum);
    }

    ret
}

fn create_prisma_client_impl(scope: &mut Scope, query_type: &DMMFOutputType) {
    let client_impl = scope.new_impl("PrismaClient");
    client_impl.new_fn("new").vis("pub").ret("PrismaClient").line("todo!()");

    // all fields that don't have a model set are added as global operations
    for query_field in query_type.fields.iter() {
        if query_field.model.is_none() {
            let function = client_impl.new_fn(&query_field.name);

            function.arg_ref_self();
            for arg in query_field.args.iter() {
                let arg_name = if &arg.name == "where" { "where_" } else { &arg.name };

                function.arg(arg_name, &map_type(&arg.input_type)).vis("pub");
            }
            function.ret(&map_type(&query_field.output_type));
            function.line("todo!()");
        }
    }
}

fn map_type(dmmf_type: &DMMFTypeInfo) -> Type {
    let x = match dmmf_type.typ.as_str() {
        "Int" => "i64",
        x => x,
    };

    if dmmf_type.is_list {
        let mut tpe = Type::new("Vec");
        tpe.generic(x);
        tpe
    } else if !dmmf_type.is_required {
        let mut tpe = Type::new("Option");
        tpe.generic(x);
        tpe
    } else {
        Type::new(x)
    }
}

fn new_pub_struct<'a>(scope: &'a mut Scope, name: &str) -> &'a mut Struct {
    scope.new_struct(name).vis("pub")
}

fn write_to_disk(content: &str) {
    let path = "/Users/marcusboehm/R/github.com/prisma/rust-client-test/src/client.rs";
    let mut file = File::create(path).expect("creating file failed");
    file.write_all(content.as_bytes()).expect("writing to file failed");
}

pub trait NameNormalizer {
    fn camel_case(&self) -> String;

    fn pascal_case(&self) -> String;
}

impl NameNormalizer for String {
    fn camel_case(&self) -> String {
        let mut c = self.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        }
    }

    fn pascal_case(&self) -> String {
        let mut c = self.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }
}

trait DmmfInputFieldExtensions {
    fn name(&self) -> String;
}

impl DmmfInputFieldExtensions for DMMFInputField {
    fn name(&self) -> String {
        if self.name == "where" {
            format!("{}_", self.name)
        } else {
            self.name.to_string()
        }
    }
}

trait DmmfSchemaExtensions {
    fn query_type(&self) -> &DMMFOutputType;
    fn mutation_type(&self) -> &DMMFOutputType;
}

impl DmmfSchemaExtensions for DMMFSchema {
    fn query_type(&self) -> &DMMFOutputType {
        self.output_types.iter().find(|ot| ot.name == "Query").unwrap()
    }

    fn mutation_type(&self) -> &DMMFOutputType {
        self.output_types.iter().find(|ot| ot.name == "Mutation").unwrap()
    }
}
