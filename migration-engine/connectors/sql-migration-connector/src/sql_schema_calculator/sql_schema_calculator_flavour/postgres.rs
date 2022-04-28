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
                            Either::Right(s) => sql::SQLOperatorClass {
                                kind: sql::SQLOperatorClassKind::Raw(s.to_string()),
                                is_default: false,
                            },
                        };

                        data.opclasses.insert(field_id, opclass);
                    }
                }
            }
        }

        context.schema.describer_schema.set_connector_data(Box::new(data));
    }
}

fn convert_opclass(opclass: OperatorClass, algo: Option<IndexAlgorithm>) -> sql::SQLOperatorClass {
    match opclass {
        OperatorClass::InetOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::InetOps,
            is_default: algo.map(|a| a.is_spgist()).unwrap_or(false),
        },
        OperatorClass::JsonbOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::JsonbOps,
            is_default: true,
        },
        OperatorClass::JsonbPathOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::JsonbPathOps,
            is_default: false,
        },
        OperatorClass::ArrayOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::ArrayOps,
            is_default: true,
        },
        OperatorClass::TextOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TextOps,
            is_default: true,
        },
        OperatorClass::BitMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::BitMinMaxOps,
            is_default: true,
        },
        OperatorClass::VarBitMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::VarBitMinMaxOps,
            is_default: true,
        },
        OperatorClass::BpcharBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::BpcharBloomOps,
            is_default: false,
        },
        OperatorClass::BpcharMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::BpcharMinMaxOps,
            is_default: true,
        },
        OperatorClass::ByteaBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::ByteaBloomOps,
            is_default: false,
        },
        OperatorClass::ByteaMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::ByteaMinMaxOps,
            is_default: true,
        },
        OperatorClass::DateBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::DateBloomOps,
            is_default: false,
        },
        OperatorClass::DateMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::DateMinMaxOps,
            is_default: true,
        },
        OperatorClass::DateMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::DateMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Float4BloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Float4BloomOps,
            is_default: false,
        },
        OperatorClass::Float4MinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Float4MinMaxOps,
            is_default: true,
        },
        OperatorClass::Float4MinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Float4MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Float8BloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Float8BloomOps,
            is_default: false,
        },
        OperatorClass::Float8MinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Float8MinMaxOps,
            is_default: true,
        },
        OperatorClass::Float8MinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Float8MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::InetInclusionOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::InetInclusionOps,
            is_default: true,
        },
        OperatorClass::InetBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::InetBloomOps,
            is_default: false,
        },
        OperatorClass::InetMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::InetMinMaxOps,
            is_default: true,
        },
        OperatorClass::InetMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::InetMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Int2BloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int2BloomOps,
            is_default: false,
        },
        OperatorClass::Int2MinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int2MinMaxOps,
            is_default: true,
        },
        OperatorClass::Int2MinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int2MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Int4BloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int4BloomOps,
            is_default: false,
        },
        OperatorClass::Int4MinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int4MinMaxOps,
            is_default: true,
        },
        OperatorClass::Int4MinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int4MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::Int8BloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int8BloomOps,
            is_default: false,
        },
        OperatorClass::Int8MinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int8MinMaxOps,
            is_default: true,
        },
        OperatorClass::Int8MinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::Int8MinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::NumericBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::NumericBloomOps,
            is_default: false,
        },
        OperatorClass::NumericMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::NumericMinMaxOps,
            is_default: true,
        },
        OperatorClass::NumericMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::NumericMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::OidBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::OidBloomOps,
            is_default: false,
        },
        OperatorClass::OidMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::OidMinMaxOps,
            is_default: true,
        },
        OperatorClass::OidMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::OidMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::TextBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TextBloomOps,
            is_default: false,
        },
        OperatorClass::TextMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TextMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimestampBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimestampBloomOps,
            is_default: false,
        },
        OperatorClass::TimestampMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimestampMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimestampMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimestampMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::TimestampTzBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimestampTzBloomOps,
            is_default: false,
        },
        OperatorClass::TimestampTzMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimestampTzMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimestampTzMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimestampTzMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::TimeBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimeBloomOps,
            is_default: false,
        },
        OperatorClass::TimeMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimeMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimeMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimeMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::TimeTzBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimeTzBloomOps,
            is_default: false,
        },
        OperatorClass::TimeTzMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimeTzMinMaxOps,
            is_default: true,
        },
        OperatorClass::TimeTzMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::TimeTzMinMaxMultiOps,
            is_default: false,
        },
        OperatorClass::UuidBloomOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::UuidBloomOps,
            is_default: false,
        },
        OperatorClass::UuidMinMaxOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::UuidMinMaxOps,
            is_default: true,
        },
        OperatorClass::UuidMinMaxMultiOps => sql::SQLOperatorClass {
            kind: sql::SQLOperatorClassKind::UuidMinMaxMultiOps,
            is_default: false,
        },
    }
}
