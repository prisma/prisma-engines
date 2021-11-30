use crate::{
    ast,
    transform::{ast_to_dml::db::types::ScalarField, helpers::ValueValidator},
};

pub(super) fn visit_native_type_attribute<'ast>(
    type_name: &'ast str,
    attr: &'ast ast::Attribute,
    scalar_field: &mut ScalarField<'ast>,
) {
    let args = &attr.arguments;
    let args: Vec<String> = args.iter().map(|arg| ValueValidator::new(&arg.value).raw()).collect();

    scalar_field.native_type = Some((type_name, args, attr.span))
}
