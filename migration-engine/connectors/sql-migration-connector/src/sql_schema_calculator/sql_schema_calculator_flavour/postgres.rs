use super::SqlSchemaCalculatorFlavour;
use crate::flavour::{PostgresFlavour, SqlFlavour};
use datamodel::{
    datamodel_connector::ScalarType,
    parser_database::{walkers::*, IndexAlgorithm, OperatorClass},
    ValidatedSchema,
};
use either::Either;
use sql::postgres::PostgresSchemaExt;
use sql_schema_describer as sql;

impl SqlSchemaCalculatorFlavour for PostgresFlavour {
    fn calculate_enums(&self, datamodel: &ValidatedSchema) -> Vec<sql::Enum> {
        datamodel
            .db
            .walk_enums()
            .map(|r#enum| sql::Enum {
                name: r#enum.database_name().to_owned(),
                values: r#enum.values().map(|val| val.database_name().to_owned()).collect(),
            })
            .collect()
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        self.datamodel_connector()
            .default_native_type_for_scalar_type(scalar_type)
    }

    fn enum_column_type(&self, field: ScalarFieldWalker<'_>, db_name: &str) -> sql::ColumnType {
        let arity = super::super::column_arity(field.ast_field().arity);

        sql::ColumnType::pure(sql::ColumnTypeFamily::Enum(db_name.to_owned()), arity)
    }

    fn push_connector_data(&self, context: &mut super::super::Context<'_>) {
        let mut data = PostgresSchemaExt::default();
        let db = &context.datamodel.db;

        for (table_idx, model) in db.walk_models().enumerate() {
            let table_id = sql::TableId(table_idx as u32);

            for (index_index, index) in model.indexes().enumerate() {
                let index_id = sql::IndexId(table_id, index_index as u32);

                for (field_idx, attrs) in index.scalar_field_attributes().enumerate() {
                    if let Some(opclass) = attrs.operator_class() {
                        let field_id = sql::IndexFieldId(index_id, field_idx as u32);

                        let opclass = match opclass.get() {
                            Either::Left(class) => convert_opclass(class, index.algorithm()),
                            Either::Right(s) => sql::postgres::SQLOperatorClass {
                                kind: sql::postgres::SQLOperatorClassKind::Raw(s.to_string()),
                                is_default: false,
                            },
                        };

                        data.opclasses.push((field_id, opclass));
                    }
                }
            }
        }

        context.schema.describer_schema.set_connector_data(Box::new(data));
    }
}

fn convert_opclass(opclass: OperatorClass, algo: Option<IndexAlgorithm>) -> sql::postgres::SQLOperatorClass {
    match opclass {
        OperatorClass::InetOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::InetOps,
            is_default: algo.map(|a| a.is_spgist()).unwrap_or(false),
        },
        OperatorClass::JsonbOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::JsonbOps,
            is_default: true,
        },
        OperatorClass::JsonbPathOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::JsonbPathOps,
            is_default: false,
        },
        OperatorClass::ArrayOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::ArrayOps,
            is_default: true,
        },
        OperatorClass::TextOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TextOps,
            is_default: true,
        },
        OperatorClass::BitMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::BitMinMaxOps,
            is_default: true,
        },
        OperatorClass::VarBitMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::VarBitMinMaxOps,
            is_default: true,
        },
        OperatorClass::BpcharBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::BpcharBloomOps,
            is_default: false,
        },
        OperatorClass::BpcharMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::BpcharMinMaxOps,
            is_default: true,
        },
        OperatorClass::ByteaBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::ByteaBloomOps,
            is_default: false,
        },
        OperatorClass::ByteaMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::ByteaMinMaxOps,
            is_default: true,
        },
        OperatorClass::DateBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::DateBloomOps,
            is_default: false,
        },
        OperatorClass::DateMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::DateMinMaxOps,
            is_default: true,
        },
        OperatorClass::DateMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::DateMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Float4BloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Float4BloomOps,
            is_default: false,
        },
        OperatorClass::Float4MinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Float4MinMaxOps,
            is_default: true,
        },
        OperatorClass::Float4MinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Float4MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Float8BloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Float8BloomOps,
            is_default: false,
        },
        OperatorClass::Float8MinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Float8MinMaxOps,
            is_default: true,
        },
        OperatorClass::Float8MinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Float8MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::InetInclusionOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::InetInclusionOps,
            is_default: true,
        },
        OperatorClass::InetBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::InetBloomOps,
            is_default: false,
        },
        OperatorClass::InetMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::InetMinMaxOps,
            is_default: true,
        },
        OperatorClass::InetMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::InetMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Int2BloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int2BloomOps,
            is_default: false,
        },
        OperatorClass::Int2MinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int2MinMaxOps,
            is_default: true,
        },
        OperatorClass::Int2MinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int2MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Int4BloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int4BloomOps,
            is_default: false,
        },
        OperatorClass::Int4MinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int4MinMaxOps,
            is_default: true,
        },
        OperatorClass::Int4MinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int4MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Int8BloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int8BloomOps,
            is_default: false,
        },
        OperatorClass::Int8MinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int8MinMaxOps,
            is_default: true,
        },
        OperatorClass::Int8MinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::Int8MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::NumericBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::NumericBloomOps,
            is_default: false,
        },
        OperatorClass::NumericMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::NumericMinMaxOps,
            is_default: true,
        },
        OperatorClass::NumericMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::NumericMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::OidBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::OidBloomOps,
            is_default: false,
        },
        OperatorClass::OidMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::OidMinMaxOps,
            is_default: true,
        },
        OperatorClass::OidMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::OidMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::TextBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TextBloomOps,
            is_default: false,
        },
        OperatorClass::TextMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TextMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimestampBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimestampBloomOps,
            is_default: false,
        },
        OperatorClass::TimestampMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimestampMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimestampMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimestampMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::TimestampTzBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimestampTzBloomOps,
            is_default: false,
        },
        OperatorClass::TimestampTzMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimestampTzMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::TimeBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimeBloomOps,
            is_default: false,
        },
        OperatorClass::TimeMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimeMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimeMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimeMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::TimeTzBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimeTzBloomOps,
            is_default: false,
        },
        OperatorClass::TimeTzMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimeTzMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimeTzMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::TimeTzMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::UuidBloomOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::UuidBloomOps,
            is_default: false,
        },
        OperatorClass::UuidMinMaxOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::UuidMinMaxOps,
            is_default: true,
        },
        OperatorClass::UuidMinMaxMultiOps => sql::postgres::SQLOperatorClass {
            kind: sql::postgres::SQLOperatorClassKind::UuidMinMaxMultiOps,
            is_default: false,
        },
    }
}
