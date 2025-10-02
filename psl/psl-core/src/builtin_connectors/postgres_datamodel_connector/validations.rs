use super::KnownPostgresType;
use crate::{
    PreviewFeature,
    builtin_connectors::PostgresType,
    datamodel_connector::{Connector, walker_ext_traits::*},
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{IndexAlgorithm, OperatorClass, ast::WithSpan, walkers::IndexWalker},
};
use enumflags2::BitFlags;

use super::PostgresDatasourceProperties;

pub(super) fn compatible_native_types(index: IndexWalker<'_>, connector: &dyn Connector, errors: &mut Diagnostics) {
    for field in index.fields() {
        if let Some(native_type) = field.native_type_instance(connector) {
            let span = field.ast_field().span();
            let PostgresType::Known(r#type) = native_type.downcast_ref() else {
                continue;
            };
            let error = connector.native_instance_error(&native_type);

            if r#type == &KnownPostgresType::Xml {
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
        let native_type = native_type_instance
            .as_ref()
            .and_then(|t| t.downcast_ref::<PostgresType>().as_known());
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
                (Some(KnownPostgresType::Inet), Some(InetOps)) => (),
                _ => err_f(native_type_name, opclass),
            }
        } else if algo.is_gin() {
            match (&native_type, opclass) {
                // Jsonb / JsonbOps + JsonbPathOps
                (None, None) if r#type.is_json() => (),

                // Array fields + ArrayOps
                (_, None) if field.as_index_field().is_list() => (),

                (Some(KnownPostgresType::JsonB), Some(JsonbOps | JsonbPathOps) | None) => (),

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
                (Some(KnownPostgresType::Inet), Some(InetOps) | None) => (),

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
                (Some(KnownPostgresType::Text), Some(TextOps) | None) => (),
                (Some(KnownPostgresType::VarChar(_)), Some(TextOps) | None) => (),
                (Some(KnownPostgresType::Char(_)), Some(TextOps) | None) => (),

                _ => err_f(native_type_name, opclass),
            }
        } else if algo.is_brin() {
            match (&native_type, opclass) {
                // Bit
                (Some(KnownPostgresType::Bit(_)), Some(BitMinMaxOps) | None) => (),

                // VarBit
                (Some(KnownPostgresType::VarBit(_)), Some(VarBitMinMaxOps) | None) => (),

                // Char
                (Some(KnownPostgresType::Char(_)), None) => (),
                (Some(KnownPostgresType::Char(_)), Some(BpcharBloomOps)) => (),
                (Some(KnownPostgresType::Char(_)), Some(BpcharMinMaxOps)) => (),

                // Date
                (Some(KnownPostgresType::Date), None) => (),
                (Some(KnownPostgresType::Date), Some(DateBloomOps)) => (),
                (Some(KnownPostgresType::Date), Some(DateMinMaxOps)) => (),
                (Some(KnownPostgresType::Date), Some(DateMinMaxMultiOps)) => (),

                // Float4
                (Some(KnownPostgresType::Real), None) => (),
                (Some(KnownPostgresType::Real), Some(Float4BloomOps)) => (),
                (Some(KnownPostgresType::Real), Some(Float4MinMaxOps)) => (),
                (Some(KnownPostgresType::Real), Some(Float4MinMaxMultiOps)) => (),

                // Float8
                (Some(KnownPostgresType::DoublePrecision), None) => (),
                (Some(KnownPostgresType::DoublePrecision), Some(Float8BloomOps)) => (),
                (Some(KnownPostgresType::DoublePrecision), Some(Float8MinMaxOps)) => (),
                (Some(KnownPostgresType::DoublePrecision), Some(Float8MinMaxMultiOps)) => (),
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
                (Some(KnownPostgresType::Inet), None) => (),
                (Some(KnownPostgresType::Inet), Some(InetInclusionOps)) => (),
                (Some(KnownPostgresType::Inet), Some(InetBloomOps)) => (),
                (Some(KnownPostgresType::Inet), Some(InetMinMaxOps)) => (),
                (Some(KnownPostgresType::Inet), Some(InetMinMaxMultiOps)) => (),

                // Int2
                (Some(KnownPostgresType::SmallInt), None) => (),
                (Some(KnownPostgresType::SmallInt), Some(Int2BloomOps)) => (),
                (Some(KnownPostgresType::SmallInt), Some(Int2MinMaxOps)) => (),
                (Some(KnownPostgresType::SmallInt), Some(Int2MinMaxMultiOps)) => (),

                // Int4
                (Some(KnownPostgresType::Integer), None) => (),
                (Some(KnownPostgresType::Integer), Some(Int4BloomOps)) => (),
                (Some(KnownPostgresType::Integer), Some(Int4MinMaxOps)) => (),
                (Some(KnownPostgresType::Integer), Some(Int4MinMaxMultiOps)) => (),
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
                (Some(KnownPostgresType::BigInt), None) => (),
                (Some(KnownPostgresType::BigInt), Some(Int8BloomOps)) => (),
                (Some(KnownPostgresType::BigInt), Some(Int8MinMaxOps)) => (),
                (Some(KnownPostgresType::BigInt), Some(Int8MinMaxMultiOps)) => (),
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
                (Some(KnownPostgresType::Decimal(_)), None) => (),
                (Some(KnownPostgresType::Decimal(_)), Some(NumericBloomOps)) => (),
                (Some(KnownPostgresType::Decimal(_)), Some(NumericMinMaxOps)) => (),
                (Some(KnownPostgresType::Decimal(_)), Some(NumericMinMaxMultiOps)) => (),
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
                (Some(KnownPostgresType::Oid), None) => (),
                (Some(KnownPostgresType::Oid), Some(OidBloomOps)) => (),
                (Some(KnownPostgresType::Oid), Some(OidMinMaxOps)) => (),
                (Some(KnownPostgresType::Oid), Some(OidMinMaxMultiOps)) => (),

                // Bytes
                (Some(KnownPostgresType::ByteA), None) => (),
                (Some(KnownPostgresType::ByteA), Some(ByteaBloomOps)) => (),
                (Some(KnownPostgresType::ByteA), Some(ByteaMinMaxOps)) => (),
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
                (Some(KnownPostgresType::Text), None) => (),
                (Some(KnownPostgresType::Text), Some(TextBloomOps)) => (),
                (Some(KnownPostgresType::Text), Some(TextMinMaxOps)) => (),
                (Some(KnownPostgresType::VarChar(_)), None) => (),
                (Some(KnownPostgresType::VarChar(_)), Some(TextBloomOps)) => (),
                (Some(KnownPostgresType::VarChar(_)), Some(TextMinMaxOps)) => (),
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
                (Some(KnownPostgresType::Timestamp(_)), None) => (),
                (Some(KnownPostgresType::Timestamp(_)), Some(TimestampBloomOps)) => (),
                (Some(KnownPostgresType::Timestamp(_)), Some(TimestampMinMaxOps)) => (),
                (Some(KnownPostgresType::Timestamp(_)), Some(TimestampMinMaxMultiOps)) => (),
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
                (Some(KnownPostgresType::Timestamptz(_)), None) => (),
                (Some(KnownPostgresType::Timestamptz(_)), Some(TimestampTzBloomOps)) => (),
                (Some(KnownPostgresType::Timestamptz(_)), Some(TimestampTzMinMaxOps)) => (),
                (Some(KnownPostgresType::Timestamptz(_)), Some(TimestampTzMinMaxMultiOps)) => (),

                // Time
                (Some(KnownPostgresType::Time(_)), None) => (),
                (Some(KnownPostgresType::Time(_)), Some(TimeBloomOps)) => (),
                (Some(KnownPostgresType::Time(_)), Some(TimeMinMaxOps)) => (),
                (Some(KnownPostgresType::Time(_)), Some(TimeMinMaxMultiOps)) => (),

                // Timetz
                (Some(KnownPostgresType::Timetz(_)), None) => (),
                (Some(KnownPostgresType::Timetz(_)), Some(TimeTzBloomOps)) => (),
                (Some(KnownPostgresType::Timetz(_)), Some(TimeTzMinMaxOps)) => (),
                (Some(KnownPostgresType::Timetz(_)), Some(TimeTzMinMaxMultiOps)) => (),

                // Uuid
                (Some(KnownPostgresType::Uuid), None) => (),
                (Some(KnownPostgresType::Uuid), Some(UuidBloomOps)) => (),
                (Some(KnownPostgresType::Uuid), Some(UuidMinMaxOps)) => (),
                (Some(KnownPostgresType::Uuid), Some(UuidMinMaxMultiOps)) => (),

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
