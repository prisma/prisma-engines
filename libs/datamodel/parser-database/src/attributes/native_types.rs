use crate::{ast, types::ScalarField};

pub(super) fn visit_native_type_attribute<'ast>(
    datasource_name: &'ast str,
    type_name: &'ast str,
    attr: &'ast ast::Attribute,
    scalar_field: &mut ScalarField<'ast>,
) {
    let args = &attr.arguments;
    let args: Vec<String> = args.arguments.iter().map(|arg| arg.value.to_string()).collect();

    scalar_field.native_type = Some((datasource_name, type_name, args, attr.span))
}
