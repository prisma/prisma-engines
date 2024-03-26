use super::{super::Context, SqlSchemaCalculatorFlavour};
use crate::flavour::{PostgresFlavour, SqlFlavour};
use either::Either;
use psl::{
    builtin_connectors::{cockroach_datamodel_connector::SequenceFunction, PostgresDatasourceProperties},
    datamodel_connector::walker_ext_traits::IndexWalkerExt,
    parser_database::{IndexAlgorithm, OperatorClass},
};
use sql::postgres::DatabaseExtension;
use sql_schema_describer::{self as sql, postgres::PostgresSchemaExt};

impl SqlSchemaCalculatorFlavour for PostgresFlavour {
    fn calculate_enums(&self, ctx: &mut Context<'_>) {
        for prisma_enum in ctx.datamodel.db.walk_enums() {
            let sql_namespace_id: sql::NamespaceId = prisma_enum
                .schema()
                .and_then(|(name, _)| ctx.schemas.get(name).cloned())
                .unwrap_or_default();
            let sql_enum_id =
                ctx.schema
                    .describer_schema
                    .push_enum(sql_namespace_id, prisma_enum.database_name().to_owned(), None);
            ctx.enum_ids.insert(prisma_enum.id, sql_enum_id);

            for value in prisma_enum.values() {
                let value_name = value.database_name().to_owned();
                ctx.schema.describer_schema.push_enum_variant(sql_enum_id, value_name);
            }
        }
    }

    fn column_default_value_for_autoincrement(&self) -> Option<sql::DefaultValue> {
        if self.is_cockroachdb() {
            Some(sql::DefaultValue::unique_rowid())
        } else {
            Some(sql::DefaultValue::sequence(""))
        }
    }

    fn push_connector_data(&self, context: &mut crate::sql_schema_calculator::Context<'_>) {
        let mut postgres_ext = PostgresSchemaExt::default();
        let db = &context.datamodel.db;

        let postgres_psl: Option<&PostgresDatasourceProperties> = context
            .datamodel
            .configuration
            .datasources
            .first()
            .and_then(|ds| ds.downcast_connector_data());

        if let Some(extensions) = postgres_psl.and_then(|props| props.extensions()) {
            for extension in extensions.extensions() {
                let name = extension
                    .db_name()
                    .to_owned()
                    .unwrap_or_else(|| extension.name())
                    .to_owned();

                let schema = extension.schema().map(|s| s.to_owned()).unwrap_or_default();
                let version = extension.version().map(|s| s.to_owned()).unwrap_or_default();

                postgres_ext.push_extension(DatabaseExtension {
                    name,
                    schema,
                    version,
                    relocatable: Default::default(),
                });
            }
        }

        for model in db.walk_models() {
            let table_id = context.model_id_to_table_id[&model.model_id()];

            // Add index algorithms and opclasses.
            for index in model.indexes() {
                let sql_index = context
                    .schema
                    .walk(table_id)
                    .indexes()
                    .find(|idx| idx.name() == index.constraint_name(self.datamodel_connector()))
                    .unwrap();

                let sql_index_algorithm = match index.algorithm() {
                    Some(IndexAlgorithm::BTree) | None => sql::postgres::SqlIndexAlgorithm::BTree,
                    Some(IndexAlgorithm::Gin) => sql::postgres::SqlIndexAlgorithm::Gin,
                    Some(IndexAlgorithm::Hash) => sql::postgres::SqlIndexAlgorithm::Hash,
                    Some(IndexAlgorithm::SpGist) => sql::postgres::SqlIndexAlgorithm::SpGist,
                    Some(IndexAlgorithm::Gist) => sql::postgres::SqlIndexAlgorithm::Gist,
                    Some(IndexAlgorithm::Brin) => sql::postgres::SqlIndexAlgorithm::Brin,
                };
                postgres_ext.indexes.push((sql_index.id, sql_index_algorithm));

                for (field_idx, attrs) in index.scalar_field_attributes().enumerate() {
                    if let Some(opclass) = attrs.operator_class() {
                        let field_id = sql_index.columns().nth(field_idx).unwrap().id;

                        let opclass = match opclass.get() {
                            Either::Left(class) => convert_opclass(class, index.algorithm()),
                            Either::Right(s) => sql::postgres::SQLOperatorClass {
                                kind: sql::postgres::SQLOperatorClassKind::Raw(s.to_owned()),
                                is_default: false,
                            },
                        };

                        postgres_ext.opclasses.push((field_id, opclass));
                    }
                }
            }

            // Add sequences for the fields with a default sequence in the model.
            for field in model.scalar_fields() {
                let field_default = if let Some(d) = field.default_value() {
                    d
                } else {
                    continue;
                };

                if !field_default.is_sequence() {
                    continue;
                }

                let mut sequence = sql::postgres::Sequence {
                    name: format!("prisma_sequence_{}_{}", model.database_name(), field.database_name()),
                    ..Default::default()
                };
                let sequence_fn = field_default.ast_attribute().arguments.arguments[0]
                    .value
                    .as_function()
                    .unwrap()
                    .1;
                let sequence_details = SequenceFunction::build_unchecked(sequence_fn);
                if let Some(start) = sequence_details.start {
                    sequence.start_value = start;
                }

                if let Some(cache) = sequence_details.cache {
                    sequence.cache_size = cache;
                }

                if let Some(max_value) = sequence_details.max_value {
                    sequence.max_value = max_value;
                }

                if let Some(min_value) = sequence_details.min_value {
                    sequence.min_value = min_value;
                }

                if let Some(increment) = sequence_details.increment {
                    sequence.increment_by = increment;
                }

                if let Some(r#virtual) = sequence_details.r#virtual {
                    sequence.r#virtual = r#virtual;
                }

                postgres_ext.sequences.push(sequence);
            }
        }

        context
            .schema
            .describer_schema
            .set_connector_data(Box::new(postgres_ext));
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
