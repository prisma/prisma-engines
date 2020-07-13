use crate::{
    error::SqlError,
    sql_renderer::IteratorJoin,
    DatabaseInfo, SqlResult,
};
use datamodel::{walkers::{ModelRef, ScalarFieldRef, TypeRef, walk_scalar_fields, walk_models}, common::*, Datamodel, WithDatabaseName, DefaultValue, ValueGenerator, ValueGeneratorFn, IndexDefinition, IndexType, FieldArity};
use prisma_models::{DatamodelConverter, TempManifestationHolder, TempRelationHolder};
use prisma_value::PrismaValue;
use quaint::prelude::SqlFamily;
use sql_schema_describer::{self as sql, ColumnArity};

pub struct SqlSchemaCalculator<'a> {
    data_model: &'a Datamodel,
    database_info: &'a DatabaseInfo,
}

impl<'a> SqlSchemaCalculator<'a> {
    pub fn calculate(data_model: &Datamodel, database_info: &DatabaseInfo) -> SqlResult<sql::SqlSchema> {
        let calculator = SqlSchemaCalculator {
            data_model,
            database_info,
        };
        calculator.calculate_internal()
    }

    fn calculate_internal(&self) -> SqlResult<sql::SqlSchema> {
        let mut tables = Vec::with_capacity(self.data_model.models.len());
        let model_tables_without_inline_relations = self.calculate_model_tables();

        for result in model_tables_without_inline_relations {
            let (model, mut table) = result?;
            self.add_inline_relations_to_model_tables(model, &mut table);
            tables.push(table);
        }

        let mut relation_tables = self.calculate_relation_tables()?;

        tables.append(&mut relation_tables);

        let enums = self.calculate_enums();
        let sequences = Vec::new();

        Ok(sql::SqlSchema {
            tables,
            enums,
            sequences,
        })
    }

    fn calculate_enums(&self) -> Vec<sql::Enum> {
        match self.database_info.sql_family() {
            SqlFamily::Postgres => self
                .data_model
                .enums()
                .map(|r#enum| sql::Enum {
                    name: r#enum.final_database_name().to_owned(),
                    values: r#enum.database_values(),
                })
                .collect(),
            SqlFamily::Mysql => {
                // This is a lower bound for the size of the generated enums (we assume each enum is
                // used at least once).
                let mut enums = Vec::with_capacity(self.data_model.enums.len());

                let enum_fields = walk_scalar_fields(&self.data_model)
                    .filter_map(|field| field.field_type().as_enum().map(|enum_ref| (field, enum_ref)));

                for (field, enum_tpe) in enum_fields {
                    let sql_enum = sql::Enum {
                        name: format!(
                            "{model_name}_{field_name}",
                            model_name = field.model().database_name(),
                            field_name = field.db_name()
                        ),
                        values: enum_tpe.r#enum.database_values(),
                    };

                    enums.push(sql_enum)
                }

                enums
            }
            _ => Vec::new(),
        }
    }

    fn calculate_model_tables<'iter>(
        &'iter self,
    ) -> impl Iterator<Item = SqlResult<(ModelRef<'a>, sql::Table)>> + 'iter {
        walk_models(self.data_model).map(move |model| {
            let columns = model
                .scalar_fields()
                .flat_map(|f| match f.field_type() {
                    TypeRef::Base(_) => Some(sql::Column {
                        name: f.db_name().to_owned(),
                        tpe: column_type(&f),
                        default: migration_value_new(&f),
                        auto_increment: matches!(f.default_value(), Some(DefaultValue::Expression(ValueGenerator { generator: ValueGeneratorFn::Autoincrement, .. }))),
                    }),
                    TypeRef::Enum(r#enum) => {
                        let enum_db_name = r#enum.db_name();
                        Some(sql::Column {
                            name: f.db_name().to_owned(),
                            tpe: enum_column_type(&f, &self.database_info, enum_db_name),
                            default: migration_value_new(&f),
                            auto_increment: false,
                        })
                    }
                    _ => None,
                })
                .collect();

            let primary_key = Some(sql::PrimaryKey {
                columns: model
                    .id_fields()
                    .map(|field| field.db_name().to_owned())
                    .collect(),
                sequence: None,
                constraint_name: None,
            }).filter(|pk| !pk.columns.is_empty());

            let single_field_indexes = model.scalar_fields().filter_map(|f| {
                if f.is_unique() {
                    Some(sql::Index {
                        name: format!("{}.{}", &model.db_name(), &f.db_name()),
                        columns: vec![f.db_name().to_owned()],
                        tpe: sql::IndexType::Unique,
                    })
                } else {
                    None
                }
            });

            let multiple_field_indexes = model.indexes().map(|index_definition: &IndexDefinition| {
                let referenced_fields: Vec<ScalarFieldRef<'_>> = index_definition
                    .fields
                    .iter()
                    .map(|field_name| model.find_scalar_field(field_name).expect("Unknown field in index directive."))
                    .collect();

                sql::Index {
                    name: index_definition.name.clone().unwrap_or_else(|| {
                        format!(
                            "{}.{}",
                            &model.db_name(),
                            referenced_fields.iter().map(|field| field.db_name()).join("_")
                        )
                    }),
                    // The model index definition uses the model field names, but the SQL Index
                    // wants the column names.
                    columns: referenced_fields
                        .iter()
                        .map(|field| field.db_name().to_owned())
                        .collect(),
                    tpe: if index_definition.tpe == IndexType::Unique {
                        sql::IndexType::Unique
                    } else {
                        sql::IndexType::Normal
                    },
                }
            });

            let table = sql::Table {
                name: model.database_name().to_owned(),
                columns,
                indices: single_field_indexes.chain(multiple_field_indexes).collect(),
                primary_key,
                foreign_keys: Vec::new(),
            };

            Ok((model, table))
        })
    }

    fn add_inline_relations_to_model_tables(&self, model: ModelRef<'a>, table: &mut sql::Table) {
        let relation_fields = model
            .relation_fields()
            .filter(|relation_field| !relation_field.is_virtual());

        for relation_field in relation_fields {
            let fk_columns: Vec<String> = relation_field.referencing_columns().map(String::from).collect();

            // Optional unique index for 1:1 relations.
            if relation_field.is_one_to_one() {
                add_one_to_one_relation_unique_index(table, &fk_columns);
            }

            // Foreign key
            {
                let fk = sql::ForeignKey {
                    constraint_name: None,
                    columns: fk_columns,
                    referenced_table: relation_field.referenced_table_name().to_owned(),
                    referenced_columns: relation_field.referenced_columns().map(String::from).collect(),
                    on_delete_action: match column_arity(relation_field.arity()) {
                        ColumnArity::Required => sql::ForeignKeyAction::Cascade,
                        _ => sql::ForeignKeyAction::SetNull,
                    },
                };

                table.foreign_keys.push(fk);
            }
        }
    }

    fn calculate_relation_tables(&self) -> SqlResult<Vec<sql::Table>> {
        let mut result = Vec::new();
        for relation in self.calculate_relations().iter() {
            if let TempManifestationHolder::Table = &relation.manifestation {
                let model_a = ModelRef::new(&relation.model_a, self.data_model);
                let model_b = ModelRef::new(&relation.model_b, self.data_model);

                let a_columns = relation_table_column(&model_a, relation.model_a_column());
                let b_columns = relation_table_column(&model_b, relation.model_b_column());

                let foreign_keys = vec![
                    sql::ForeignKey {
                        constraint_name: None,
                        columns: a_columns.iter().map(|col| col.name.clone()).collect(),
                        referenced_table: model_a.db_name().to_owned(),
                        referenced_columns: first_unique_criterion(model_a)
                            .map_err(SqlError::Generic)?
                            .into_iter()
                            .map(|field| field.db_name().to_owned())
                            .collect(),
                        on_delete_action: sql::ForeignKeyAction::Cascade,
                    },
                    sql::ForeignKey {
                        constraint_name: None,
                        columns: b_columns.iter().map(|col| col.name.clone()).collect(),
                        referenced_table: model_b.db_name().to_owned(),
                        referenced_columns: first_unique_criterion(model_b)
                            .map_err(SqlError::Generic)?
                            .into_iter()
                            .map(|field| field.db_name().to_owned())
                            .collect(),
                        on_delete_action: sql::ForeignKeyAction::Cascade,
                    },
                ];

                let mut columns = a_columns;
                columns.extend(b_columns.iter().map(|col| col.to_owned()));

                let indexes = vec![
                    sql::Index {
                        name: format!("{}_AB_unique", relation.table_name()),
                        columns: columns.iter().map(|col| col.name.clone()).collect(),
                        tpe: sql::IndexType::Unique,
                    },
                    sql::Index {
                        name: format!("{}_B_index", relation.table_name()),
                        columns: b_columns.into_iter().map(|col| col.name).collect(),
                        tpe: sql::IndexType::Normal,
                    },
                ];

                let table = sql::Table {
                    name: relation.table_name(),
                    columns,
                    indices: indexes,
                    primary_key: None,
                    foreign_keys,
                };
                result.push(table);
            }
        }
        Ok(result)
    }

    fn calculate_relations(&self) -> Vec<TempRelationHolder> {
        DatamodelConverter::calculate_relations(&self.data_model)
    }
}

fn relation_table_column(referenced_model: &ModelRef<'_>, reference_field_name: String) -> Vec<sql::Column> {
    let unique_field = referenced_model.scalar_fields().find(|f| f.is_unique());
    let id_field = referenced_model.scalar_fields().find(|f| f.is_id());

    let unique_field = id_field
        .or(unique_field)
        .unwrap_or_else(|| panic!("No unique criteria found in model {}", &referenced_model.name()));

    vec![sql::Column {
        name: reference_field_name,
        tpe: column_type(&unique_field),
        default: None,
        auto_increment: false,
    }]
}

fn migration_value_new(field: &ScalarFieldRef<'_>) -> Option<sql_schema_describer::DefaultValue> {
    if field.is_id() {
        return None;
    }

    let value = match &field.default_value()? {
        datamodel::DefaultValue::Single(s) => match field.field_type() {
            TypeRef::Enum(inum) => {
                let corresponding_value = inum
                    .r#enum
                    .values()
                    .find(|val| val.name.as_str() == s.to_string())
                    .expect("could not find enum value");

                PrismaValue::Enum(corresponding_value.final_database_name().to_owned())
            }
            _ => s.clone(),
        },
        datamodel::DefaultValue::Expression(expression) if expression.name == "now" && expression.args.is_empty() => {
            return Some(sql_schema_describer::DefaultValue::NOW)
        }
        datamodel::DefaultValue::Expression(expression) if expression.name == "dbgenerated" && expression.args.is_empty() => {
            return Some(sql_schema_describer::DefaultValue::DBGENERATED(String::new()))
        }
        datamodel::DefaultValue::Expression(_) => return None,
    };

    Some(sql_schema_describer::DefaultValue::VALUE(value))
}

fn enum_column_type(field: &ScalarFieldRef<'_>, database_info: &DatabaseInfo, db_name: &str) -> sql::ColumnType {
    let arity = column_arity(field.arity());
    match database_info.sql_family() {
        SqlFamily::Postgres => sql::ColumnType::pure(sql::ColumnTypeFamily::Enum(db_name.to_owned()), arity),
        SqlFamily::Mysql => sql::ColumnType::pure(
            sql::ColumnTypeFamily::Enum(format!("{}_{}", field.model().db_name(), field.db_name())),
            arity,
        ),
        _ => column_type(field),
    }
}

fn column_type(field: &ScalarFieldRef<'_>) -> sql::ColumnType {
    column_type_for_scalar_type(&scalar_type_for_field(field), column_arity(field.arity()))
}

fn scalar_type_for_field(field: &ScalarFieldRef<'_>) -> ScalarType {
    match field.field_type() {
        TypeRef::Base(ref scalar) => *scalar,
        TypeRef::Enum(_) => ScalarType::String,
        x => panic!(format!(
            "This field type is not suported here. Field type is {:?} on field {}",
            x,
            field.name()
        )),
    }
}

fn column_arity(arity: FieldArity) -> sql::ColumnArity {
    match &arity {
        FieldArity::Required => sql::ColumnArity::Required,
        FieldArity::List => sql::ColumnArity::List,
        FieldArity::Optional => sql::ColumnArity::Nullable,
    }
}

fn column_type_for_scalar_type(scalar_type: &ScalarType, column_arity: ColumnArity) -> sql::ColumnType {
    match scalar_type {
        ScalarType::Int => sql::ColumnType::pure(sql::ColumnTypeFamily::Int, column_arity),
        ScalarType::Float => sql::ColumnType::pure(sql::ColumnTypeFamily::Float, column_arity),
        ScalarType::Boolean => sql::ColumnType::pure(sql::ColumnTypeFamily::Boolean, column_arity),
        ScalarType::String => sql::ColumnType::pure(sql::ColumnTypeFamily::String, column_arity),
        ScalarType::DateTime => sql::ColumnType::pure(sql::ColumnTypeFamily::DateTime, column_arity),
        ScalarType::Json => sql::ColumnType::pure(sql::ColumnTypeFamily::Json, column_arity),
    }
}

fn add_one_to_one_relation_unique_index(table: &mut sql::Table, column_names: &[String]) {
    // Don't add a duplicate index.
    if table
        .indices
        .iter()
        .any(|index| index.columns == column_names && index.tpe.is_unique())
    {
        return;
    }

    let columns_suffix = column_names.join("_");
    let index = sql::Index {
        name: format!("{}_{}", table.name, columns_suffix),
        columns: column_names.to_owned(),
        tpe: sql::IndexType::Unique,
    };

    table.indices.push(index);
}

/// This should match the logic in `prisma_models::Model::primary_identifier`.
fn first_unique_criterion(model: ModelRef<'_>) -> anyhow::Result<Vec<ScalarFieldRef<'_>>> {
    // First candidate: the primary key.
    {
        let id_fields: Vec<_> = model.id_fields().collect();

        if !id_fields.is_empty() {
            return Ok(id_fields);
        }
    }

    // Second candidate: a required scalar field with a unique index.
    {
        let first_scalar_unique_required_field = model
            .scalar_fields()
            .find(|field| field.is_unique() && field.is_required());

        if let Some(field) = first_scalar_unique_required_field {
            return Ok(vec![field]);
        }
    }

    // Third candidate: any multi-field unique constraint.
    {
        let first_multi_field_unique = model.unique_indexes().next();

        if let Some(index) = first_multi_field_unique {
            return Ok(index.fields().collect());
        }
    }

    anyhow::bail!("Could not find the first unique criteria on model {}", model.name());
}
