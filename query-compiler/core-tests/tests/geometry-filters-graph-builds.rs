use std::sync::Arc;

use query_core::{QueryDocument, QueryGraphBuilder};
use request_handlers::{JsonBody, JsonSingleQuery, RequestBody};

#[test]
fn geometry_near_filter_builds_query_graph() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        generator client {
            provider = "prisma-client"
        }

        model Location {
            id       Int                    @id @default(autoincrement())
            position Geometry(Point, 4326)?
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "Location",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "position": {
                        "near": {
                            "point": [2.35, 48.85],
                            "maxDistance": 100000
                        }
                    }
                }
            },
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema).unwrap();

    let QueryDocument::Single(query) = doc else {
        panic!("expected single query");
    };

    QueryGraphBuilder::new(&query_schema)
        .build(query)
        .expect("findMany with near filter should compile to a query graph");
}

#[test]
fn geometry_within_filter_builds_query_graph() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model Location {
            id       Int                    @id @default(autoincrement())
            position Geometry(Point, 4326)?
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "Location",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "position": {
                        "within": {
                            "polygon": [
                                [0, 0],
                                [0, 2],
                                [2, 2],
                                [2, 0],
                                [0, 0]
                            ]
                        }
                    }
                }
            },
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema).unwrap();

    let QueryDocument::Single(query) = doc else {
        panic!("expected single query");
    };

    QueryGraphBuilder::new(&query_schema)
        .build(query)
        .expect("findMany with within filter should compile to a query graph");
}

#[test]
fn geometry_orderby_distance_builds_query_graph() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model Location {
            id       Int                    @id @default(autoincrement())
            position Geometry(Point, 4326)?
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "Location",
        "action": "findMany",
        "query": {
            "arguments": {
                "orderBy": [
                    {
                        "position": {
                            "distanceFrom": {
                                "point": [0, 0],
                                "direction": "asc"
                            }
                        }
                    }
                ]
            },
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema).unwrap();

    let QueryDocument::Single(query) = doc else {
        panic!("expected single query");
    };

    QueryGraphBuilder::new(&query_schema)
        .build(query)
        .expect("findMany with distanceFrom orderBy should compile to a query graph");
}

#[test]
fn geometry_combined_filter_and_orderby_builds_query_graph() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model Location {
            id       Int                    @id @default(autoincrement())
            position Geometry(Point, 4326)?
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "Location",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "position": {
                        "near": {
                            "point": [0, 0],
                            "maxDistance": 50000
                        }
                    }
                },
                "orderBy": [
                    {
                        "position": {
                            "distanceFrom": {
                                "point": [0, 0],
                                "direction": "asc"
                            }
                        }
                    }
                ]
            },
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema).unwrap();

    let QueryDocument::Single(query) = doc else {
        panic!("expected single query");
    };

    QueryGraphBuilder::new(&query_schema)
        .build(query)
        .expect("findMany with geometry filter and orderBy should compile to a query graph");
}

#[test]
fn geometry_not_filter_builds_query_graph() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model Location {
            id       Int                    @id @default(autoincrement())
            position Geometry(Point, 4326)?
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "Location",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "NOT": {
                        "position": {
                            "near": {
                                "point": [0, 0],
                                "maxDistance": 10000
                            }
                        }
                    }
                }
            },
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema).unwrap();

    let QueryDocument::Single(query) = doc else {
        panic!("expected single query");
    };

    QueryGraphBuilder::new(&query_schema)
        .build(query)
        .expect("findMany with NOT geometry filter should compile to a query graph");
}

#[test]
fn geometry_or_filter_builds_query_graph() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model Location {
            id       Int                    @id @default(autoincrement())
            position Geometry(Point, 4326)?
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "Location",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "OR": [
                        {
                            "position": {
                                "near": {
                                    "point": [0, 0],
                                    "maxDistance": 10000
                                }
                            }
                        },
                        {
                            "position": {
                                "near": {
                                    "point": [10, 10],
                                    "maxDistance": 5000
                                }
                            }
                        }
                    ]
                }
            },
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema).unwrap();

    let QueryDocument::Single(query) = doc else {
        panic!("expected single query");
    };

    QueryGraphBuilder::new(&query_schema)
        .build(query)
        .expect("findMany with OR geometry filter should compile to a query graph");
}

#[test]
fn geometry_custom_srid_builds_query_graph() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model LocationMercator {
            id       Int                    @id @default(autoincrement())
            position Geometry(Point, 3857)?
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "LocationMercator",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "position": {
                        "near": {
                            "point": [1000000, 6000000],
                            "maxDistance": 5000,
                            "srid": 3857
                        }
                    }
                }
            },
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema).unwrap();

    let QueryDocument::Single(query) = doc else {
        panic!("expected single query");
    };

    QueryGraphBuilder::new(&query_schema)
        .build(query)
        .expect("findMany with custom SRID 3857 should compile to a query graph");
}
