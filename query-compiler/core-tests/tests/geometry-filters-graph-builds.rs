use std::sync::Arc;

use query_core::{QueryDocument, QueryGraphBuilder, with_sync_unevaluated_request_context};
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
            position Geometry? @db.Geometry(Point, 4326)
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
            position Geometry? @db.Geometry(Point, 4326)
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
            position Geometry? @db.Geometry(Point, 4326)
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
            position Geometry? @db.Geometry(Point, 4326)
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
            position Geometry? @db.Geometry(Point, 4326)
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
            position Geometry? @db.Geometry(Point, 4326)
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
            position Geometry? @db.Geometry(Point, 3857)
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

/// Combining cursor-based pagination with `orderBy: { position: { distance: { ... } } }` has no
/// deterministic SQL translation (the distance reference point is not part of the row). The
/// extractor must reject the combination with a clear `InputError` instead of letting it reach
/// the SQL builder where it would panic with `unimplemented!()`.
#[test]
fn cursor_with_geometry_orderby_is_rejected() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model Location {
            id       Int      @id @default(autoincrement())
            position Geometry @db.Geometry(Point, 4326)
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
                "cursor": { "id": 1 },
                "orderBy": [
                    { "position": { "distanceFrom": { "point": [0, 0], "direction": "asc" } } }
                ]
            },
            "selection": {
                "id": true,
                "position": true
            }
        }
    }"#;

    with_sync_unevaluated_request_context(|| {
        let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
        let request = RequestBody::Json(JsonBody::Single(query));
        let doc = request.into_doc(&query_schema).unwrap();

        let QueryDocument::Single(query) = doc else {
            panic!("expected single query");
        };

        let error = QueryGraphBuilder::new(&query_schema)
            .build(query)
            .expect_err("cursor + geometry orderBy must be rejected");

        let message = format!("{error}");
        assert!(
            message.contains("Cursor-based pagination") && message.to_ascii_lowercase().contains("geometry"),
            "unexpected error message: {message}",
        );
    });
}

/// Malformed GeoJSON in an `intersects` filter must surface as a user-facing `InputError` instead
/// of crashing the builder (older code used `panic!()` for polygons missing the closing vertex).
#[test]
fn intersects_with_malformed_polygon_returns_input_error() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model Location {
            id       Int      @id @default(autoincrement())
            position Geometry @db.Geometry(Point, 4326)
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    // Polygon ring with only two distinct positions — auto-close cannot rescue it, and the
    // extractor must surface a parse error rather than panicking.
    let query_json = r#"{
        "modelName": "Location",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "position": {
                        "intersects": {
                            "geometry": {
                                "type": "Polygon",
                                "coordinates": [[[0, 0], [1, 1]]]
                            },
                            "srid": 4326
                        }
                    }
                }
            },
            "selection": { "id": true }
        }
    }"#;

    with_sync_unevaluated_request_context(|| {
        let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
        let request = RequestBody::Json(JsonBody::Single(query));
        let doc = request.into_doc(&query_schema).unwrap();

        let QueryDocument::Single(query) = doc else {
            panic!("expected single query");
        };

        let error = QueryGraphBuilder::new(&query_schema)
            .build(query)
            .expect_err("malformed polygon must be rejected");

        let message = format!("{error}");
        assert!(
            message.to_ascii_lowercase().contains("polygon"),
            "expected error to mention the polygon ring, got: {message}",
        );
    });
}

/// `intersects` filter must reject `Multi*` / `GeometryCollection` GeoJSON shapes at extraction
/// time. The previous behaviour silently emitted a `false` predicate inside the SQL builder, which
/// made empty result sets indistinguishable from "filter ignored". This regression test pins the
/// fast-fail path so future contributors don't reintroduce the silent fallback.
#[test]
fn intersects_with_unsupported_geojson_returns_input_error() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        model Location {
            id       Int      @id @default(autoincrement())
            position Geometry @db.Geometry(Point, 4326)
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
                        "intersects": {
                            "geometry": {
                                "type": "MultiPoint",
                                "coordinates": [[0.0, 0.0], [1.0, 1.0]]
                            },
                            "srid": 4326
                        }
                    }
                }
            },
            "selection": { "id": true }
        }
    }"#;

    with_sync_unevaluated_request_context(|| {
        let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
        let request = RequestBody::Json(JsonBody::Single(query));
        let doc = request.into_doc(&query_schema).unwrap();

        let QueryDocument::Single(query) = doc else {
            panic!("expected single query");
        };

        let error = QueryGraphBuilder::new(&query_schema)
            .build(query)
            .expect_err("MultiPoint geometry must be rejected by the extractor");

        let message = format!("{error}");
        assert!(
            message.contains("MultiPoint"),
            "expected error to mention the unsupported GeoJSON type, got: {message}",
        );
    });
}

/// SevInf #1/#4: `Geography` is a first-class PSL scalar type. The geometry filter pipeline must
/// accept fields declared as `Geography @db.Geography(...)` end-to-end and route them through the
/// same near/within/intersects machinery as `Geometry` fields.
#[test]
fn geography_field_supports_full_filter_pipeline() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        generator client {
            provider = "prisma-client"
        }

        model Place {
            id       Int       @id @default(autoincrement())
            location Geography @db.Geography(Point, 4326)
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let query_json = r#"{
        "modelName": "Place",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "location": {
                        "near": {
                            "point": [2.35, 48.85],
                            "maxDistance": 100000
                        }
                    }
                },
                "orderBy": [
                    { "location": { "distanceFrom": { "point": [0, 0], "direction": "asc" } } }
                ]
            },
            "selection": {
                "id": true,
                "location": true
            }
        }
    }"#;

    with_sync_unevaluated_request_context(|| {
        let query: JsonSingleQuery = serde_json::from_str(query_json).unwrap();
        let request = RequestBody::Json(JsonBody::Single(query));
        let doc = request.into_doc(&query_schema).unwrap();

        let QueryDocument::Single(query) = doc else {
            panic!("expected single query");
        };

        QueryGraphBuilder::new(&query_schema)
            .build(query)
            .expect("Geography field with near + orderBy should compile to a query graph");
    });
}

/// SevInf #1/#4: `Geometry @db.Geography(...)` (and the symmetric mismatch) is rejected at PSL
/// validation time so the rest of the toolchain never has to reason about an inconsistent
/// `geometry`/`geography` declaration.
#[test]
fn mismatched_geometry_geography_native_attribute_is_rejected() {
    let schema_string = r#"
        datasource db {
            provider = "postgresql"
        }

        generator client {
            provider = "prisma-client"
        }

        model Bad {
            id   Int      @id
            loc  Geometry @db.Geography(Point, 4326)
        }
    "#;

    let schema = psl::validate_without_extensions(schema_string.into());
    let messages: Vec<String> = schema
        .diagnostics
        .errors()
        .iter()
        .map(|e| e.message().to_owned())
        .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("Native type Geography is not compatible with declared field type Geometry")),
        "expected mismatch error, got: {messages:?}",
    );
}
