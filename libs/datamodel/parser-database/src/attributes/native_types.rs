use crate::{
    ast,
    types::{CompositeTypeField, ScalarField},
    StringId,
};

pub(super) fn visit_model_field_native_type_attribute(
    datasource_name: StringId,
    type_name: StringId,
    attr: &ast::Attribute,
    scalar_field: &mut ScalarField,
) {
    let args = &attr.arguments;
    let args: Vec<String> = args.arguments.iter().map(|arg| arg.value.to_string()).collect();

    scalar_field.native_type = Some((datasource_name, type_name, args, attr.span))
}

pub(super) fn visit_composite_type_field_native_type_attribute(
    datasource_name: StringId,
    type_name: StringId,
    attr: &ast::Attribute,
    composite_type_field: &mut CompositeTypeField,
) {
    let args = &attr.arguments;
    let args: Vec<String> = args.arguments.iter().map(|arg| arg.value.to_string()).collect();

    composite_type_field.native_type = Some((datasource_name, type_name, args, attr.span))
}
