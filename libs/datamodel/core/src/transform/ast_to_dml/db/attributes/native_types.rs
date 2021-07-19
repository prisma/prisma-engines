use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::{
        ast_to_dml::db::{context::Context, types::ScalarField},
        helpers::ValueValidator,
    },
};
use datamodel_connector::connector_error::{ConnectorError, ErrorKind};
use dml::scalars::ScalarType;
use itertools::Itertools;

pub(super) fn visit_native_type_attribute<'ast>(
    type_name: &'ast str,
    attr: &'ast ast::Attribute,
    scalar_type: ScalarType,
    scalar_field: &mut ScalarField<'ast>,
    ctx: &mut Context<'ast>,
) {
    let args = &attr.arguments;
    let diagnostics = &mut ctx.diagnostics;

    // convert arguments to string if possible
    let args: Vec<String> = args.iter().map(|arg| ValueValidator::new(&arg.value).raw()).collect();

    let constructor = if let Some(cons) = ctx.db.active_connector().find_native_type_constructor(type_name) {
        cons
    } else {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::NativeTypeNameUnknown {
                native_type: type_name.to_owned(),
                connector_name: ctx
                    .db
                    .datasource()
                    .map(|ds| ds.active_provider.clone())
                    .unwrap_or_else(|| "Default".to_owned()),
            })
            .to_string(),
            attr.span,
        ));
        return;
    };

    let number_of_args = args.len();

    if number_of_args < constructor._number_of_args
        || ((number_of_args > constructor._number_of_args) && constructor._number_of_optional_args == 0)
    {
        diagnostics.push_error(DatamodelError::new_argument_count_missmatch_error(
            type_name,
            constructor._number_of_args,
            number_of_args,
            attr.span,
        ));
        return;
    }

    if number_of_args > constructor._number_of_args + constructor._number_of_optional_args
        && constructor._number_of_optional_args > 0
    {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::OptionalArgumentCountMismatchError {
                native_type: type_name.to_owned(),
                optional_count: constructor._number_of_optional_args,
                given_count: number_of_args,
            })
            .to_string(),
            attr.span,
        ));
        return;
    }

    // check for compatibility with scalar type
    if !constructor.prisma_types.contains(&scalar_type) {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::IncompatibleNativeType {
                native_type: type_name.to_owned(),
                field_type: scalar_type.to_string(),
                expected_types: constructor.prisma_types.iter().map(|s| s.to_string()).join(" or "),
            })
            .to_string(),
            attr.span,
        ));
        return;
    }

    if let Err(connector_error) = ctx.db.active_connector().parse_native_type(type_name, args.clone()) {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &connector_error.to_string(),
            attr.span,
        ));
        return;
    };

    scalar_field.native_type = Some((type_name, args))
}
