mod datamodel_helpers;

use crate::{error::SqlError, sql_renderer::IteratorJoin, DatabaseInfo, SqlResult};
use chrono::*;
use datamodel::common::*;
use datamodel::*;
use datamodel_helpers::{FieldRef, ModelRef, TypeRef};
use prisma_models::{DatamodelConverter, TempManifestationHolder, TempRelationHolder};
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
        let mut tables = Vec::new();
        let model_tables_without_inline_relations = self.calculate_model_tables()?;
        let mut model_tables = self.add_inline_relations_to_model_tables(model_tables_without_inline_relations)?;
        let mut relation_tables = self.calculate_relation_tables()?;

        tables.append(&mut model_tables);
        tables.append(&mut relation_tables);

        // guarantee same sorting as in the sql-schema-describer
        for table in &mut tables {
            table
                .columns
                .sort_unstable_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        }

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
                    name: r#enum
                        .single_database_name()
                        .map(|s| s.to_owned())
                        .unwrap_or_else(|| r#enum.name.clone()),
                    values: r#enum.values.clone(),
                })
                .collect(),
            SqlFamily::Mysql => {
                // This is a lower bound for the size of the generated enums (we assume each enum is
                // used at least once).
                let mut enums = Vec::with_capacity(self.data_model.enums.len());

                let enum_fields = datamodel_helpers::walk_fields(&self.data_model)
                    .filter_map(|field| field.field_type().as_enum().map(|enum_ref| (field, enum_ref)));

                for (field, enum_tpe) in enum_fields {
                    let sql_enum = sql::Enum {
                        name: format!(
                            "{model_name}_{field_name}",
                            model_name = field.model().database_name(),
                            field_name = field.db_name()
                        ),
                        values: enum_tpe.r#enum.values.clone(),
                    };

                    enums.push(sql_enum)
                }

                enums
            }
            _ => Vec::new(),
        }
    }

    fn calculate_model_tables(&self) -> SqlResult<Vec<ModelTable>> {
        datamodel_helpers::walk_models(self.data_model)
            .map(|model| {
                let columns = model
                    .fields()
                    .flat_map(|f| match f.field_type() {
                        TypeRef::Base(_) => Some(sql::Column {
                            name: f.db_name().to_owned(),
                            tpe: column_type(&f),
                            default: migration_value_new(&f),
                            auto_increment: {
                                match f.default_value() {
                                    Some(DefaultValue::Expression(ValueGenerator {
                                        name: _,
                                        args: _,
                                        generator: ValueGeneratorFn::Autoincrement,
                                    })) => true,
                                    _ => false,
                                }
                            },
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

                let primary_key = sql::PrimaryKey {
                    columns: model.id_fields().map(|field| field.db_name().to_owned()).collect(),
                    sequence: None,
                };

                let single_field_indexes = model.fields().filter_map(|f| {
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
                    let referenced_fields: Vec<FieldRef> = index_definition
                        .fields
                        .iter()
                        .map(|field_name| model.find_field(field_name).expect("Unknown field in index directive."))
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
                    primary_key: Some(primary_key),
                    foreign_keys: Vec::new(),
                };

                Ok(ModelTable {
                    model: model.model().clone(),
                    table,
                })
            })
            .collect()
    }

    fn add_inline_relations_to_model_tables(&self, model_tables: Vec<ModelTable>) -> SqlResult<Vec<sql::Table>> {
        let mut result = Vec::new();
        let relations = self.calculate_relations();
        for mut model_table in model_tables {
            for relation in relations.iter() {
                match &relation.manifestation {
                    TempManifestationHolder::Inline {
                        in_table_of_model,
                        field: dml_field,
                        referenced_fields,
                    } if in_table_of_model == &model_table.model.name => {
                        let (model, related_model) = if model_table.model == relation.model_a {
                            (&relation.model_a, &relation.model_b)
                        } else {
                            (&relation.model_b, &relation.model_a)
                        };

                        let (model, related_model) = (
                            ModelRef {
                                model: &model,
                                datamodel: self.data_model,
                            },
                            ModelRef {
                                model: &related_model,
                                datamodel: self.data_model,
                            },
                        );

                        let field = model.fields().find(|f| &f.name() == &dml_field.name).unwrap();

                        let referenced_fields: Vec<FieldRef> = if referenced_fields.is_empty() {
                            first_unique_criterion(related_model).map_err(SqlError::Generic)?
                        } else {
                            let fields: Vec<_> = related_model
                                .fields()
                                .filter(|field| {
                                    referenced_fields
                                        .iter()
                                        .any(|referenced| referenced.as_str() == field.name())
                                })
                                .collect();

                            if fields.len() != referenced_fields.len() {
                                return Err(crate::SqlError::Generic(anyhow::anyhow!(
                                    "References to unknown fields {referenced_fields:?} on `{model_name}`",
                                    model_name = related_model.name(),
                                    referenced_fields = referenced_fields,
                                )));
                            }

                            fields
                        };

                        let columns: Vec<sql::Column> = field
                            .field
                            .data_source_fields
                            .iter()
                            .map(|dsf| sql::Column {
                                name: dsf.name.clone(),
                                tpe: column_type_for_scalar_type(&dsf.field_type, column_arity(dsf.arity)),
                                default: None,
                                auto_increment: false,
                            })
                            .collect();

                        let foreign_key = sql::ForeignKey {
                            constraint_name: None,
                            columns: columns.iter().map(|col| col.name.to_owned()).collect(),
                            referenced_table: related_model.db_name().to_owned(),
                            referenced_columns: referenced_fields
                                .iter()
                                .map(|referenced_field| referenced_field.db_name().to_owned())
                                .collect(),
                            on_delete_action: match column_arity(field.arity()) {
                                ColumnArity::Required => sql::ForeignKeyAction::Restrict,
                                _ => sql::ForeignKeyAction::SetNull,
                            },
                        };

                        if relation.is_one_to_one() {
                            add_one_to_one_relation_unique_index(&mut model_table.table, &columns)
                        }

                        model_table.table.columns.extend(columns);
                        model_table.table.foreign_keys.push(foreign_key);
                    }
                    _ => {}
                }
            }
            result.push(model_table.table);
        }
        Ok(result)
    }

    fn calculate_relation_tables(&self) -> SqlResult<Vec<sql::Table>> {
        let mut result = Vec::new();
        for relation in self.calculate_relations().iter() {
            match &relation.manifestation {
                TempManifestationHolder::Table => {
                    let model_a = ModelRef {
                        datamodel: self.data_model,
                        model: &relation.model_a,
                    };
                    let model_b = ModelRef {
                        datamodel: self.data_model,
                        model: &relation.model_b,
                    };
                    let a_columns = relation_table_columns(&model_a, relation.model_a_column());
                    let mut b_columns = relation_table_columns(&model_b, relation.model_b_column());

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
                    columns.append(&mut b_columns);

                    let index = sql::Index {
                        // TODO: rename
                        name: format!("{}_AB_unique", relation.table_name()),
                        columns: columns.iter().map(|col| col.name.clone()).collect(),
                        tpe: sql::IndexType::Unique,
                    };

                    let table = sql::Table {
                        name: relation.table_name(),
                        columns,
                        indices: vec![index],
                        primary_key: None,
                        foreign_keys,
                    };
                    result.push(table);
                }
                _ => {}
            }
        }
        Ok(result)
    }

    fn calculate_relations(&self) -> Vec<TempRelationHolder> {
        DatamodelConverter::calculate_relations(&self.data_model)
    }
}

fn relation_table_columns(referenced_model: &ModelRef<'_>, reference_field_name: String) -> Vec<sql::Column> {
    // TODO: must also work with multi field unique
    if referenced_model.model().id_fields.is_empty() {
        let unique_field = referenced_model.fields().find(|f| f.is_unique());
        let id_field = referenced_model.fields().find(|f| f.is_id());

        let unique_field = id_field.or(unique_field).expect(&format!(
            "No unique criteria found in model {}",
            &referenced_model.name()
        ));

        vec![sql::Column {
            name: reference_field_name,
            tpe: column_type(&unique_field),
            default: None,
            auto_increment: false,
        }]
    } else {
        referenced_model
            .id_fields()
            .map(|referenced_field| sql::Column {
                name: format!(
                    "{reference_field_name}_{referenced_column_name}",
                    reference_field_name = reference_field_name,
                    referenced_column_name = referenced_field.db_name()
                ),
                tpe: column_type(&referenced_field),
                default: None,
                auto_increment: false,
            })
            .collect()
    }
}

#[derive(PartialEq, Debug)]
struct ModelTable {
    table: sql::Table,
    model: Model,
}

fn migration_value_new(field: &FieldRef<'_>) -> Option<String> {
    let value = match (&field.default_value(), field.arity()) {
        (Some(df), _) => match df {
            dml::DefaultValue::Single(s) => s.clone(),
            dml::DefaultValue::Expression(_) => default_migration_value(&field.field_type()),
        },
        // This is a temporary hack until we can report impossible unexecutable migrations.
        (None, FieldArity::Required) => default_migration_value(&field.field_type()),
        (None, _) => return None,
    };

    let result = match value {
        ScalarValue::Boolean(x) => {
            if x {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        ScalarValue::Int(x) => format!("{}", x),
        ScalarValue::Float(x) => format!("{}", x),
        ScalarValue::Decimal(x) => format!("{}", x),
        ScalarValue::String(x) => format!("{}", x),

        ScalarValue::DateTime(x) => {
            let mut raw = format!("{}", x); // this will produce a String 1970-01-01 00:00:00 UTC
            raw.truncate(raw.len() - 4); // strip the UTC suffix
            format!("{}", raw)
        }
        ScalarValue::ConstantLiteral(x) => format!("{}", x), // this represents enum values
    };

    if field.is_id() {
        None
    } else {
        Some(result)
    }
}

fn default_migration_value(field_type: &TypeRef<'_>) -> ScalarValue {
    match field_type {
        TypeRef::Base(ScalarType::Boolean) => ScalarValue::Boolean(false),
        TypeRef::Base(ScalarType::Int) => ScalarValue::Int(0),
        TypeRef::Base(ScalarType::Float) => ScalarValue::Float(0.0),
        TypeRef::Base(ScalarType::String) => ScalarValue::String("".to_string()),
        TypeRef::Base(ScalarType::Decimal) => ScalarValue::Decimal(0.0),
        TypeRef::Base(ScalarType::DateTime) => {
            let naive = NaiveDateTime::from_timestamp(0, 0);
            let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
            ScalarValue::DateTime(datetime)
        }
        TypeRef::Enum(inum) => {
            let first_value = inum
                .values()
                .first()
                .expect(&format!("Enum {} did not contain any values.", inum.name()));
            ScalarValue::String(first_value.to_string())
        }
        _ => unimplemented!("this functions must only be called for scalar fields"),
    }
}

fn enum_column_type(field: &FieldRef<'_>, database_info: &DatabaseInfo, db_name: &str) -> sql::ColumnType {
    let arity = column_arity(field.arity());
    match database_info.sql_family() {
        SqlFamily::Postgres => sql::ColumnType::pure(sql::ColumnTypeFamily::Enum(db_name.to_owned()), arity),
        SqlFamily::Mysql => sql::ColumnType::pure(
            sql::ColumnTypeFamily::Enum(format!("{}_{}", field.model().name(), field.name())),
            arity,
        ),
        _ => column_type(field),
    }
}

fn column_type(field: &FieldRef<'_>) -> sql::ColumnType {
    column_type_for_scalar_type(&scalar_type_for_field(field), column_arity(field.arity()))
}

fn scalar_type_for_field(field: &FieldRef<'_>) -> ScalarType {
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
        ScalarType::Decimal => unimplemented!(),
    }
}

fn add_one_to_one_relation_unique_index(table: &mut sql::Table, columns: &Vec<sql::Column>) {
    let column_names: Vec<String> = columns.iter().map(|c| c.name.to_owned()).collect();
    let columns_suffix = column_names.join("_");
    let index = sql::Index {
        name: format!("{}_{}", table.name, columns_suffix),
        columns: column_names,
        tpe: sql::IndexType::Unique,
    };

    table.indices.push(index);
}

/// This should match the logic in `prisma_models::Model::primary_identifier`.
fn first_unique_criterion<'a>(model: ModelRef<'a>) -> anyhow::Result<Vec<FieldRef<'a>>> {
    // First candidate: the primary key.
    {
        let id_fields: Vec<_> = model.id_fields().collect();

        if !id_fields.is_empty() {
            return Ok(id_fields);
        }
    }

    // Second candidate: a required scalar field with a unique index.
    {
        let first_scalar_unique_required_field = model.fields().find(|field| field.is_unique() && field.is_required());

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
