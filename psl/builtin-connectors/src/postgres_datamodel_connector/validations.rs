use crate::PostgresType;
use enumflags2::BitFlags;
use psl_core::{
    datamodel_connector::{walker_ext_traits::*, Connector},
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{ast::WithSpan, walkers::IndexWalker, IndexAlgorithm, OperatorClass},
    PreviewFeature,
};

use super::PostgresDatasourceProperties;

pub(super) fn compatible_native_types(index: IndexWalker<'_>, connector: &dyn Connector, errors: &mut Diagnostics) {
    for field in index.fields() {
        if let Some(native_type) = field.native_type_instance(connector) {
            let span = field.ast_field().span();
            let r#type: &PostgresType = native_type.downcast_ref();
            let error = connector.native_instance_error(&native_type);

            if r#type == &PostgresType::Xml {
                if index.is_unique() {
                    errors.push_error(error.new_incompatible_native_type_with_unique("", span))
                } else {
                    errors.push_error(error.new_incompatible_native_type_with_index("", span))
                };

                break;
            }
        }
    }
}

/// Cannot have more than one column in SP-GiST indices.
pub(super) fn spgist_indexed_column_count(index: IndexWalker<'_>, errors: &mut Diagnostics) {
    if !matches!(index.algorithm(), Some(IndexAlgorithm::SpGist)) {
        return;
    }

    if index.fields().len() == 1 {
        return;
    }

    errors.push_error(DatamodelError::new_attribute_validation_error(
        "SpGist does not support multi-column indices.",
        index.attribute_name(),
        index.ast_attribute().span,
    ));
}

/// Validating the correct usage of GiST/GIN/SP-GiST and BRIN indices.
pub(super) fn generalized_index_validations(
    index: IndexWalker<'_>,
    connector: &dyn Connector,
    errors: &mut Diagnostics,
) {
    use OperatorClass::*;

    let algo = index.algorithm().unwrap_or(IndexAlgorithm::BTree);

    for field in index.scalar_field_attributes() {
        // No validation for `raw` needed.
        if field.operator_class().map(|c| c.get().is_right()).unwrap_or(false) {
            continue;
        }

        let native_type_instance = field.as_index_field().native_type_instance(connector);
        let native_type: Option<&PostgresType> = native_type_instance.as_ref().map(|t| t.downcast_ref());
        let native_type_name = native_type_instance
            .as_ref()
            .map(|nt| connector.native_type_to_parts(nt).0);

        let r#type = field.as_index_field().scalar_field_type();

        let opclass = field.operator_class().and_then(|c| c.get().left());

        match opclass {
            Some(opclass) if !opclass.supports_index_type(algo) => {
                let msg =
                    format!("The given operator class `{opclass}` is not supported with the `{algo}` index type.");

                errors.push_error(DatamodelError::new_attribute_validation_error(
                    &msg,
                    index.attribute_name(),
                    index.ast_attribute().span,
                ));

                continue;
            }
            _ => (),
        }

        let mut err_f = |native_type_name: Option<&str>, opclass| match (native_type_name, opclass) {
            (Some(native_type), Some(opclass)) => {
                let name = field.as_index_field().name();

                let msg = format!(
                    "The given operator class `{opclass}` does not support native type `{native_type}` of field `{name}`."
                );

                errors.push_error(DatamodelError::new_attribute_validation_error(
                    &msg,
                    index.attribute_name(),
                    index.ast_attribute().span,
                ));
            }
            (Some(native_type), None) => {
                let msg = format!("The {algo} index field type `{native_type}` has no default operator class.");

                errors.push_error(DatamodelError::new_attribute_validation_error(
                    &msg,
                    index.attribute_name(),
                    index.ast_attribute().span,
                ));
            }
            (None, Some(opclass)) => {
                let name = field.as_index_field().name();
                let msg = format!(
                    "The given operator class `{opclass}` expects the field `{name}` to define a valid native type."
                );

                errors.push_error(DatamodelError::new_attribute_validation_error(
                    &msg,
                    index.attribute_name(),
                    index.ast_attribute().span,
                ));
            }
            _ => {
                if !algo.supports_field_type(field.as_index_field()) {
                    let name = field.as_index_field().name();
                    let msg = format!("The {algo} index type does not support the type of the field `{name}`.");

                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &msg,
                        index.attribute_name(),
                        index.ast_attribute().span,
                    ));
                }
            }
        };

        if algo.is_gist() {
            match (&native_type, opclass) {
                // Inet / InetOps
                (Some(PostgresType::Inet), Some(InetOps)) => (),
                _ => err_f(native_type_name, opclass),
            }
        } else if algo.is_gin() {
            match (&native_type, opclass) {
                // Jsonb / JsonbOps + JsonbPathOps
                (None, None) if r#type.is_json() => (),

                // Array fields + ArrayOps
                (_, None) if field.as_index_field().is_list() => (),

                (Some(PostgresType::JsonB), Some(JsonbOps | JsonbPathOps) | None) => (),

                (None, Some(JsonbOps | JsonbPathOps)) => {
                    if !r#type.is_json() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                        "The given operator class `{opclass}` points to the field `{name}` that is not of Json type."
                    );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }

                // any array / ArrayOps
                (_, Some(ArrayOps)) => {
                    if field
                        .as_index_field()
                        .as_scalar_field()
                        .filter(|sf| !sf.ast_field().arity.is_list())
                        .is_none()
                    {
                        continue;
                    }

                    let name = field.as_index_field().name();

                    let msg = format!(
                        "The given operator class `ArrayOps` expects the type of field `{name}` to be an array."
                    );

                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &msg,
                        index.attribute_name(),
                        index.ast_attribute().span,
                    ));
                }
                _ => err_f(native_type_name, opclass),
            }
        } else if algo.is_spgist() {
            match (&native_type, opclass) {
                // Inet
                (Some(PostgresType::Inet), Some(InetOps) | None) => (),

                // Text / TextOps
                (None, None) if r#type.is_string() => (),
                (None, Some(TextOps)) => {
                    if !r#type.is_string() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                            "The given operator class `{opclass}` points to the field `{name}` that is not of String type."
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }
                (Some(PostgresType::Text), Some(TextOps) | None) => (),
                (Some(PostgresType::VarChar(_)), Some(TextOps) | None) => (),
                (Some(PostgresType::Char(_)), Some(TextOps) | None) => (),

                _ => err_f(native_type_name, opclass),
            }
        } else if algo.is_brin() {
            match (&native_type, opclass) {
                // Bit
                (Some(PostgresType::Bit(_)), Some(BitMinMaxOps) | None) => (),

                // VarBit
                (Some(PostgresType::VarBit(_)), Some(VarBitMinMaxOps) | None) => (),

                // Char
                (Some(PostgresType::Char(_)), None) => (),
                (Some(PostgresType::Char(_)), Some(BpcharBloomOps)) => (),
                (Some(PostgresType::Char(_)), Some(BpcharMinMaxOps)) => (),

                // Date
                (Some(PostgresType::Date), None) => (),
                (Some(PostgresType::Date), Some(DateBloomOps)) => (),
                (Some(PostgresType::Date), Some(DateMinMaxOps)) => (),
                (Some(PostgresType::Date), Some(DateMinMaxMultiOps)) => (),

                // Float4
                (Some(PostgresType::Real), None) => (),
                (Some(PostgresType::Real), Some(Float4BloomOps)) => (),
                (Some(PostgresType::Real), Some(Float4MinMaxOps)) => (),
                (Some(PostgresType::Real), Some(Float4MinMaxMultiOps)) => (),

                // Float8
                (Some(PostgresType::DoublePrecision), None) => (),
                (Some(PostgresType::DoublePrecision), Some(Float8BloomOps)) => (),
                (Some(PostgresType::DoublePrecision), Some(Float8MinMaxOps)) => (),
                (Some(PostgresType::DoublePrecision), Some(Float8MinMaxMultiOps)) => (),
                (None, None) if r#type.is_float() => (),
                (None, Some(Float8BloomOps | Float8MinMaxOps | Float8MinMaxMultiOps)) => {
                    if !r#type.is_float() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                            "The given operator class `{opclass}` points to the field `{name}` that is not of Float type."
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }

                // Inet
                (Some(PostgresType::Inet), None) => (),
                (Some(PostgresType::Inet), Some(InetInclusionOps)) => (),
                (Some(PostgresType::Inet), Some(InetBloomOps)) => (),
                (Some(PostgresType::Inet), Some(InetMinMaxOps)) => (),
                (Some(PostgresType::Inet), Some(InetMinMaxMultiOps)) => (),

                // Int2
                (Some(PostgresType::SmallInt), None) => (),
                (Some(PostgresType::SmallInt), Some(Int2BloomOps)) => (),
                (Some(PostgresType::SmallInt), Some(Int2MinMaxOps)) => (),
                (Some(PostgresType::SmallInt), Some(Int2MinMaxMultiOps)) => (),

                // Int4
                (Some(PostgresType::Integer), None) => (),
                (Some(PostgresType::Integer), Some(Int4BloomOps)) => (),
                (Some(PostgresType::Integer), Some(Int4MinMaxOps)) => (),
                (Some(PostgresType::Integer), Some(Int4MinMaxMultiOps)) => (),
                (None, None) if r#type.is_int() => (),
                (None, Some(Int4BloomOps | Int4MinMaxOps | Int4MinMaxMultiOps)) => {
                    if !r#type.is_int() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                            "The given operator class `{opclass}` points to the field `{name}` that is not of Int type."
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }

                // Int8
                (Some(PostgresType::BigInt), None) => (),
                (Some(PostgresType::BigInt), Some(Int8BloomOps)) => (),
                (Some(PostgresType::BigInt), Some(Int8MinMaxOps)) => (),
                (Some(PostgresType::BigInt), Some(Int8MinMaxMultiOps)) => (),
                (None, None) if r#type.is_bigint() => (),
                (None, Some(Int8BloomOps | Int8MinMaxOps | Int8MinMaxMultiOps)) => {
                    if !r#type.is_bigint() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                            "The given operator class `{opclass}` points to the field `{name}` that is not of BigInt type."
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }

                // Numeric
                (Some(PostgresType::Decimal(_)), None) => (),
                (Some(PostgresType::Decimal(_)), Some(NumericBloomOps)) => (),
                (Some(PostgresType::Decimal(_)), Some(NumericMinMaxOps)) => (),
                (Some(PostgresType::Decimal(_)), Some(NumericMinMaxMultiOps)) => (),
                (None, None) if r#type.is_decimal() => (),
                (None, Some(NumericBloomOps | NumericMinMaxOps | NumericMinMaxMultiOps)) => {
                    if !r#type.is_decimal() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                            "The given operator class `{opclass}` points to the field `{name}` that is not of Decimal type."
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }

                // Oid
                (Some(PostgresType::Oid), None) => (),
                (Some(PostgresType::Oid), Some(OidBloomOps)) => (),
                (Some(PostgresType::Oid), Some(OidMinMaxOps)) => (),
                (Some(PostgresType::Oid), Some(OidMinMaxMultiOps)) => (),

                // Bytes
                (Some(PostgresType::ByteA), None) => (),
                (Some(PostgresType::ByteA), Some(ByteaBloomOps)) => (),
                (Some(PostgresType::ByteA), Some(ByteaMinMaxOps)) => (),
                (None, None) if r#type.is_bytes() => (),
                (None, Some(ByteaBloomOps | ByteaMinMaxOps)) => {
                    if !r#type.is_bytes() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                            "The given operator class `{opclass}` points to the field `{name}` that is not of Bytes type."
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }

                // Text
                (Some(PostgresType::Text), None) => (),
                (Some(PostgresType::Text), Some(TextBloomOps)) => (),
                (Some(PostgresType::Text), Some(TextMinMaxOps)) => (),
                (Some(PostgresType::VarChar(_)), None) => (),
                (Some(PostgresType::VarChar(_)), Some(TextBloomOps)) => (),
                (Some(PostgresType::VarChar(_)), Some(TextMinMaxOps)) => (),
                (None, None) if r#type.is_string() => (),
                (None, Some(TextBloomOps | TextMinMaxOps)) => {
                    if !r#type.is_string() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                            "The given operator class `{opclass}` points to the field `{name}` that is not of String type."
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }

                // Timestamp
                (Some(PostgresType::Timestamp(_)), None) => (),
                (Some(PostgresType::Timestamp(_)), Some(TimestampBloomOps)) => (),
                (Some(PostgresType::Timestamp(_)), Some(TimestampMinMaxOps)) => (),
                (Some(PostgresType::Timestamp(_)), Some(TimestampMinMaxMultiOps)) => (),
                (None, None) if r#type.is_datetime() => (),
                (None, Some(TimestampBloomOps | TimestampMinMaxOps | TimestampMinMaxMultiOps)) => {
                    if !r#type.is_datetime() {
                        let name = field.as_index_field().name();
                        let opclass = opclass.unwrap();

                        let msg = format!(
                            "The given operator class `{opclass}` points to the field `{name}` that is not of DateTime type."
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &msg,
                            index.attribute_name(),
                            index.ast_attribute().span,
                        ));
                    }
                }

                // Timestamptz
                (Some(PostgresType::Timestamptz(_)), None) => (),
                (Some(PostgresType::Timestamptz(_)), Some(TimestampTzBloomOps)) => (),
                (Some(PostgresType::Timestamptz(_)), Some(TimestampTzMinMaxOps)) => (),
                (Some(PostgresType::Timestamptz(_)), Some(TimestampTzMinMaxMultiOps)) => (),

                // Time
                (Some(PostgresType::Time(_)), None) => (),
                (Some(PostgresType::Time(_)), Some(TimeBloomOps)) => (),
                (Some(PostgresType::Time(_)), Some(TimeMinMaxOps)) => (),
                (Some(PostgresType::Time(_)), Some(TimeMinMaxMultiOps)) => (),

                // Timetz
                (Some(PostgresType::Timetz(_)), None) => (),
                (Some(PostgresType::Timetz(_)), Some(TimeTzBloomOps)) => (),
                (Some(PostgresType::Timetz(_)), Some(TimeTzMinMaxOps)) => (),
                (Some(PostgresType::Timetz(_)), Some(TimeTzMinMaxMultiOps)) => (),

                // Uuid
                (Some(PostgresType::Uuid), None) => (),
                (Some(PostgresType::Uuid), Some(UuidBloomOps)) => (),
                (Some(PostgresType::Uuid), Some(UuidMinMaxOps)) => (),
                (Some(PostgresType::Uuid), Some(UuidMinMaxMultiOps)) => (),

                _ => err_f(native_type_name, opclass),
            }
        }
    }
}

pub(super) fn extensions_preview_flag_must_be_set(
    preview_features: BitFlags<PreviewFeature>,
    props: &PostgresDatasourceProperties,
    errors: &mut Diagnostics,
) {
    if preview_features.contains(PreviewFeature::PostgresqlExtensions) {
        return;
    }

    let span = match props.extensions() {
        Some(extensions) => extensions.span,
        None => return,
    };

    errors.push_error(DatamodelError::new_static(
        "The `extensions` property is only available with the `postgresqlExtensions` preview feature.",
        span,
    ));
}

pub(super) fn extension_names_follow_prisma_syntax_rules(
    preview_features: BitFlags<PreviewFeature>,
    props: &PostgresDatasourceProperties,
    errors: &mut Diagnostics,
) {
    if !preview_features.contains(PreviewFeature::PostgresqlExtensions) {
        return;
    }

    let extensions = match props.extensions() {
        Some(extensions) => extensions,
        None => return,
    };

    // Sadly these rules are already in identifier validation. It is
    // not easy to share those rules here due to the code
    // organization. TODO: organize the code better!
    for extension in extensions.extensions() {
        if extension.name.is_empty() {
            errors.push_error(DatamodelError::new_validation_error(
                "The name of an extension must not be empty.",
                extension.span,
            ));
        } else if extension.name.chars().next().unwrap().is_numeric() {
            errors.push_error(DatamodelError::new_validation_error(
                "The name of an extension must not start with a number.",
                extension.span,
            ));
        } else if extension.name.contains('-') {
            errors.push_error(DatamodelError::new_validation_error(
                "The character `-` is not allowed in extension names.",
                extension.span,
            ))
        }
    }
}
