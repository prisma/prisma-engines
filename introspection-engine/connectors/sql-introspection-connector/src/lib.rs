pub mod calculate_datamodel; // only exported to be able to unit test it
mod commenting_out_guardrails;
mod error;
mod introspection;
mod introspection_helpers;
mod prisma_1_defaults;
mod re_introspection;
mod sanitize_datamodel_names;
mod schema_describer_loading;
mod version_checker;
mod warnings;

use datamodel::Datamodel;
pub use error::*;
use introspection_connector::{
    ConnectorError, ConnectorResult, DatabaseMetadata, IntrospectionConnector, IntrospectionResult,
};
use quaint::prelude::ConnectionInfo;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::{fmt, future::Future};
use tracing_futures::Instrument;

pub type SqlIntrospectionResult<T> = core::result::Result<T, SqlError>;

pub struct SqlIntrospectionConnector {
    connection_info: ConnectionInfo,
    describer: Box<dyn SqlSchemaDescriberBackend>,
}

impl fmt::Debug for SqlIntrospectionConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqlIntrospectionConnector")
            .field("connection_info", &self.connection_info)
            .field("describer", &"Box<dyn SqlSchemaDescriberBackend>")
            .finish()
    }
}

impl SqlIntrospectionConnector {
    pub async fn new(url: &str) -> ConnectorResult<SqlIntrospectionConnector> {
        let (describer, connection_info) = schema_describer_loading::load_describer(&url)
            .instrument(tracing::debug_span!("Loading describer"))
            .await
            .map_err(|error| {
                ConnectionInfo::from_url(url)
                    .map(|connection_info| error.into_connector_error(&connection_info))
                    .unwrap_or_else(|err| ConnectorError::url_parse_error(err, url))
            })?;

        tracing::debug!("SqlIntrospectionConnector initialized.");

        Ok(SqlIntrospectionConnector {
            describer,
            connection_info,
        })
    }

    async fn catch<O>(&self, fut: impl Future<Output = Result<O, SqlError>>) -> ConnectorResult<O> {
        fut.await
            .map_err(|sql_introspection_error| sql_introspection_error.into_connector_error(&self.connection_info))
    }

    async fn list_databases_internal(&self) -> SqlIntrospectionResult<Vec<String>> {
        Ok(self.describer.list_databases().await?)
    }

    async fn get_metadata_internal(&self) -> SqlIntrospectionResult<DatabaseMetadata> {
        let sql_metadata = self.describer.get_metadata(self.connection_info.schema_name()).await?;
        let db_metadate = DatabaseMetadata {
            table_count: sql_metadata.table_count,
            size_in_bytes: sql_metadata.size_in_bytes,
        };
        Ok(db_metadate)
    }

    async fn describe(&self) -> SqlIntrospectionResult<SqlSchema> {
        Ok(self.describer.describe(self.connection_info.schema_name()).await?)
    }

    async fn version(&self) -> SqlIntrospectionResult<String> {
        Ok(self
            .describer
            .version(self.connection_info.schema_name())
            .await?
            .unwrap_or_else(|| "Database version information not available.".into()))
    }
}

#[async_trait::async_trait]
impl IntrospectionConnector for SqlIntrospectionConnector {
    async fn list_databases(&self) -> ConnectorResult<Vec<String>> {
        Ok(self.catch(self.list_databases_internal()).await?)
    }

    async fn get_metadata(&self) -> ConnectorResult<DatabaseMetadata> {
        Ok(self.catch(self.get_metadata_internal()).await?)
    }

    async fn get_database_description(&self) -> ConnectorResult<String> {
        let sql_schema = self.catch(self.describe()).await?;
        tracing::debug!("SQL Schema Describer is done: {:?}", sql_schema);
        let description = format!("{:#?}", sql_schema);
        Ok(description)
    }

    async fn get_database_version(&self) -> ConnectorResult<String> {
        let sql_schema = self.catch(self.version()).await?;
        tracing::debug!("Fetched db version for: {:?}", sql_schema);
        let description = serde_json::to_string(&sql_schema).unwrap();
        Ok(description)
    }
    async fn introspect(&self, previous_data_model: &Datamodel) -> ConnectorResult<IntrospectionResult> {
        // let sql_schema = self.catch(self.describe()).await?;
        // tracing::debug!("SQL Schema Describer is done: {:?}", sql_schema);
        let dm = r#"{
    "tables": [{
            "name": "_FollowRelation",
            "columns": [{
                    "name": "A",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "B",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                }
            ],
            "indices": [{
                    "name": "_FollowRelation_AB_unique",
                    "columns": [
                        "A",
                        "B"
                    ],
                    "tpe": "Unique"
                },
                {
                    "name": "_FollowRelation_B_index",
                    "columns": [
                        "B"
                    ],
                    "tpe": "Normal"
                }
            ],
            "primary_key": null,
            "foreign_keys": [{
                    "constraint_name": "_followrelation_ibfk_1",
                    "columns": [
                        "A"
                    ],
                    "referenced_table": "User",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                },
                {
                    "constraint_name": "_followrelation_ibfk_2",
                    "columns": [
                        "B"
                    ],
                    "referenced_table": "User",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                }
            ]
        },
        {
            "name": "_Migration",
            "columns": [{
                    "name": "revision",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": true
                },
                {
                    "name": "name",
                    "tpe": {
                        "full_data_type": "text",
                        "family": "String",
                        "arity": "Required",
                        "native_type": "Text"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "datamodel",
                    "tpe": {
                        "full_data_type": "longtext",
                        "family": "String",
                        "arity": "Required",
                        "native_type": "LongText"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "status",
                    "tpe": {
                        "full_data_type": "text",
                        "family": "String",
                        "arity": "Required",
                        "native_type": "Text"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "applied",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "rolled_back",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "datamodel_steps",
                    "tpe": {
                        "full_data_type": "longtext",
                        "family": "String",
                        "arity": "Required",
                        "native_type": "LongText"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "database_migration",
                    "tpe": {
                        "full_data_type": "longtext",
                        "family": "String",
                        "arity": "Required",
                        "native_type": "LongText"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "errors",
                    "tpe": {
                        "full_data_type": "longtext",
                        "family": "String",
                        "arity": "Required",
                        "native_type": "LongText"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "started_at",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "finished_at",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Nullable",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": null,
                    "auto_increment": false
                }
            ],
            "indices": [],
            "primary_key": {
                "columns": [
                    "revision"
                ],
                "sequence": null,
                "constraint_name": null
            },
            "foreign_keys": []
        },
        {
            "name": "_RoomToUser",
            "columns": [{
                    "name": "A",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "B",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                }
            ],
            "indices": [{
                    "name": "_RoomToUser_AB_unique",
                    "columns": [
                        "A",
                        "B"
                    ],
                    "tpe": "Unique"
                },
                {
                    "name": "_RoomToUser_B_index",
                    "columns": [
                        "B"
                    ],
                    "tpe": "Normal"
                }
            ],
            "primary_key": null,
            "foreign_keys": [{
                    "constraint_name": "_roomtouser_ibfk_1",
                    "columns": [
                        "A"
                    ],
                    "referenced_table": "Room",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                },
                {
                    "constraint_name": "_roomtouser_ibfk_2",
                    "columns": [
                        "B"
                    ],
                    "referenced_table": "User",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                }
            ]
        },
        {
            "name": "Comment",
            "columns": [{
                    "name": "id",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": true
                },
                {
                    "name": "text",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Required",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "createdAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "updatedAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "userId",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "postId",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                }
            ],
            "indices": [{
                    "name": "postId",
                    "columns": [
                        "postId"
                    ],
                    "tpe": "Normal"
                },
                {
                    "name": "userId",
                    "columns": [
                        "userId"
                    ],
                    "tpe": "Normal"
                }
            ],
            "primary_key": {
                "columns": [
                    "id"
                ],
                "sequence": null,
                "constraint_name": null
            },
            "foreign_keys": [{
                    "constraint_name": "comment_ibfk_2",
                    "columns": [
                        "postId"
                    ],
                    "referenced_table": "Post",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                },
                {
                    "constraint_name": "comment_ibfk_1",
                    "columns": [
                        "userId"
                    ],
                    "referenced_table": "User",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                }
            ]
        },
        {
            "name": "File",
            "columns": [{
                    "name": "id",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": true
                },
                {
                    "name": "url",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Required",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "postId",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                }
            ],
            "indices": [{
                "name": "postId",
                "columns": [
                    "postId"
                ],
                "tpe": "Normal"
            }],
            "primary_key": {
                "columns": [
                    "id"
                ],
                "sequence": null,
                "constraint_name": null
            },
            "foreign_keys": [{
                "constraint_name": "file_ibfk_1",
                "columns": [
                    "postId"
                ],
                "referenced_table": "Post",
                "referenced_columns": [
                    "id"
                ],
                "on_delete_action": "Cascade",
                "on_update_action": "Cascade"
            }]
        },
        {
            "name": "Like",
            "columns": [{
                    "name": "id",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": true
                },
                {
                    "name": "userId",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "postId",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "createdAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "updatedAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                }
            ],
            "indices": [{
                    "name": "postId",
                    "columns": [
                        "postId"
                    ],
                    "tpe": "Normal"
                },
                {
                    "name": "userId",
                    "columns": [
                        "userId"
                    ],
                    "tpe": "Normal"
                }
            ],
            "primary_key": {
                "columns": [
                    "id"
                ],
                "sequence": null,
                "constraint_name": null
            },
            "foreign_keys": [{
                    "constraint_name": "like_ibfk_2",
                    "columns": [
                        "postId"
                    ],
                    "referenced_table": "Post",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                },
                {
                    "constraint_name": "like_ibfk_1",
                    "columns": [
                        "userId"
                    ],
                    "referenced_table": "User",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                }
            ]
        },
        {
            "name": "Message",
            "columns": [{
                    "name": "id",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": true
                },
                {
                    "name": "text",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Required",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "createdAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "updatedAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "roomId",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "userId",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                }
            ],
            "indices": [{
                    "name": "roomId",
                    "columns": [
                        "roomId"
                    ],
                    "tpe": "Normal"
                },
                {
                    "name": "userId",
                    "columns": [
                        "userId"
                    ],
                    "tpe": "Normal"
                }
            ],
            "primary_key": {
                "columns": [
                    "id"
                ],
                "sequence": null,
                "constraint_name": null
            },
            "foreign_keys": [{
                    "constraint_name": "message_ibfk_3",
                    "columns": [
                        "roomId"
                    ],
                    "referenced_table": "Room",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                },
                {
                    "constraint_name": "message_ibfk_4",
                    "columns": [
                        "userId"
                    ],
                    "referenced_table": "User",
                    "referenced_columns": [
                        "id"
                    ],
                    "on_delete_action": "Cascade",
                    "on_update_action": "Cascade"
                }
            ]
        },
        {
            "name": "Post",
            "columns": [{
                    "name": "id",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": true
                },
                {
                    "name": "location",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Nullable",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "caption",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Required",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "userId",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "createdAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "updatedAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                }
            ],
            "indices": [{
                "name": "userId",
                "columns": [
                    "userId"
                ],
                "tpe": "Normal"
            }],
            "primary_key": {
                "columns": [
                    "id"
                ],
                "sequence": null,
                "constraint_name": null
            },
            "foreign_keys": [{
                "constraint_name": "post_ibfk_1",
                "columns": [
                    "userId"
                ],
                "referenced_table": "User",
                "referenced_columns": [
                    "id"
                ],
                "on_delete_action": "Cascade",
                "on_update_action": "Cascade"
            }]
        },
        {
            "name": "Room",
            "columns": [{
                    "name": "id",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": true
                },
                {
                    "name": "createdAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "updatedAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                }
            ],
            "indices": [],
            "primary_key": {
                "columns": [
                    "id"
                ],
                "sequence": null,
                "constraint_name": null
            },
            "foreign_keys": []
        },
        {
            "name": "User",
            "columns": [{
                    "name": "id",
                    "tpe": {
                        "full_data_type": "int",
                        "family": "Int",
                        "arity": "Required",
                        "native_type": "Int"
                    },
                    "default": null,
                    "auto_increment": true
                },
                {
                    "name": "avatar",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Nullable",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": {
                        "kind": { "VALUE": "" },
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "email",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Required",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "userName",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Required",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "firstName",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Nullable",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": {
                        "kind": { "VALUE": "" },
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "lastName",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Nullable",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": {
                        "kind": { "VALUE": "" },
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "bio",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Nullable",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": {
                        "kind": { "VALUE": "" },
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "loginSecret",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Nullable",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                },
                {
                    "name": "createdAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "updatedAt",
                    "tpe": {
                        "full_data_type": "datetime(3)",
                        "family": "DateTime",
                        "arity": "Required",
                        "native_type": {
                            "DateTime": 3
                        }
                    },
                    "default": {
                        "kind": "NOW",
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "confirmSecret",
                    "tpe": {
                        "full_data_type": "tinyint(1)",
                        "family": "Boolean",
                        "arity": "Required",
                        "native_type": "TinyInt"
                    },
                    "default": {
                        "kind": { "VALUE": false },
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "password",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Nullable",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": {
                        "kind": { "VALUE": "" },
                        "constraint_name": null
                    },
                    "auto_increment": false
                },
                {
                    "name": "facebookId",
                    "tpe": {
                        "full_data_type": "varchar(191)",
                        "family": "String",
                        "arity": "Nullable",
                        "native_type": {
                            "VarChar": 191
                        }
                    },
                    "default": null,
                    "auto_increment": false
                }
            ],
            "indices": [{
                    "name": "User.email_unique",
                    "columns": [
                        "email"
                    ],
                    "tpe": "Unique"
                },
                {
                    "name": "User.facebookId_unique",
                    "columns": [
                        "facebookId"
                    ],
                    "tpe": "Unique"
                },
                {
                    "name": "User.userName_unique",
                    "columns": [
                        "userName"
                    ],
                    "tpe": "Unique"
                }
            ],
            "primary_key": {
                "columns": [
                    "id"
                ],
                "sequence": null,
                "constraint_name": null
            },
            "foreign_keys": []
        }
    ],
    "enums": [],
    "sequences": [],
    "views": [],
    "procedures": []
}"#;

        let sql_schema: SqlSchema = serde_json::from_str(dm).unwrap();

        let family = self.connection_info.sql_family();

        let introspection_result = calculate_datamodel::calculate_datamodel(&sql_schema, &family, &previous_data_model)
            .map_err(|sql_introspection_error| sql_introspection_error.into_connector_error(&self.connection_info))?;

        tracing::debug!("Calculating datamodel is done: {:?}", introspection_result.data_model);

        Ok(introspection_result)
    }
}

trait Dedup<T: PartialEq + Clone> {
    fn clear_duplicates(&mut self);
}

impl<T: PartialEq + Clone> Dedup<T> for Vec<T> {
    fn clear_duplicates(&mut self) {
        let mut already_seen = vec![];
        self.retain(|item| match already_seen.contains(item) {
            true => false,
            _ => {
                already_seen.push(item.clone());
                true
            }
        })
    }
}
