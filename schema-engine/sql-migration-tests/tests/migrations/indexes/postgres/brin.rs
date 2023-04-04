use sql_migration_tests::test_api::*;
use sql_schema_describer::postgres::{SQLOperatorClassKind, SqlIndexAlgorithm};

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_change_from_btree(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int

          @@index([data])
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SqlIndexAlgorithm::BTree))
    });

    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| idx.assert_algorithm(SqlIndexAlgorithm::Brin))
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_bit_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Bit(1)

          @@index([data(ops: BitMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::BitMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_bit_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Bit(1)

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::BitMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_varbit_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.VarBit

          @@index([data(ops: VarBitMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::VarBitMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_varbit_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.VarBit

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::VarBitMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_bpchar_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Char(1)

          @@index([data(ops: BpcharMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::BpcharMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_bpchar_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Char(1)

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::BpcharMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_bpchar_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data String? @db.Char(1)

          @@index([data(ops: BpcharBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::BpcharBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_bytea_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data Bytes?

          @@index([data(ops: ByteaMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::ByteaMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_bytea_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Bytes?

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::ByteaMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_bytea_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data Bytes?

          @@index([data(ops: ByteaBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::ByteaBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_date_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Date

          @@index([data(ops: DateMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::DateMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_date_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Date

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::DateMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_date_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Date

          @@index([data(ops: DateBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::DateBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_date_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Date

          @@index([data(ops: DateMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::DateMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_real_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Float @db.Real

          @@index([data(ops: Float4MinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Float4MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_real_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Float @db.Real

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Float4MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_real_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Float @db.Real

          @@index([data(ops: Float4BloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Float4BloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_real_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Float @db.Real

          @@index([data(ops: Float4MinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::Float4MinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_double_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Float

          @@index([data(ops: Float8MinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Float8MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_double_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Float

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Float8MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_double_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Float

          @@index([data(ops: Float8BloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Float8BloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_double_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int   @id @default(autoincrement())
          data Float

          @@index([data(ops: Float8MinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::Float8MinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_inet_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Inet

          @@index([data(ops: InetMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::InetMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_inet_inclusion_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Inet

          @@index([data(ops: InetInclusionOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::InetInclusionOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_inet_inclusion_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Inet

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::InetInclusionOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_inet_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Inet

          @@index([data(ops: InetBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::InetBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_inet_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Inet

          @@index([data(ops: InetMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::InetMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_int2_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int @db.SmallInt

          @@index([data(ops: Int2MinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int2MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_int2_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int @db.SmallInt

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int2MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_int2_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int @db.SmallInt

          @@index([data(ops: Int2BloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int2BloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_int2_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int @db.SmallInt

          @@index([data(ops: Int2MinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::Int2MinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_int4_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int

          @@index([data(ops: Int4MinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int4MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_int4_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int4MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_int4_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int

          @@index([data(ops: Int4BloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int4BloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_int4_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int

          @@index([data(ops: Int4MinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::Int4MinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_int8_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data BigInt

          @@index([data(ops: Int8MinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int8MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_int8_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data BigInt

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int8MinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_int8_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data BigInt

          @@index([data(ops: Int8BloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::Int8BloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_int8_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data BigInt

          @@index([data(ops: Int8MinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::Int8MinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_numeric_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data Decimal

          @@index([data(ops: NumericMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::NumericMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_numeric_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data Decimal

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::NumericMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_numeric_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data Decimal

          @@index([data(ops: NumericBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::NumericBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_numeric_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int     @id @default(autoincrement())
          data Decimal

          @@index([data(ops: NumericMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::NumericMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_oid_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int @db.Oid

          @@index([data(ops: OidMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::OidMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_oid_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int @db.Oid

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::OidMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_oid_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int @db.Oid

          @@index([data(ops: OidBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::OidBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_oid_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int @id @default(autoincrement())
          data Int @db.Oid

          @@index([data(ops: OidMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::OidMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_text_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String

          @@index([data(ops: TextMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_text_minmax_ops_varchar(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.VarChar(420)

          @@index([data(ops: TextMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_text_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_text_minmax_ops_default_varchar(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.VarChar(420)

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_text_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String

          @@index([data(ops: TextBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_text_bloom_ops_varchar(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.VarChar(420)

          @@index([data(ops: TextBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TextBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_timestamp_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime

          @@index([data(ops: TimestampMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimestampMinMaxOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_timestamp_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimestampMinMaxOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_timestamp_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime

          @@index([data(ops: TimestampBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimestampBloomOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_timestamp_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime

          @@index([data(ops: TimestampMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimestampMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_timestamptz_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Timestamptz

          @@index([data(ops: TimestampTzMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimestampTzMinMaxOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_timestamptz_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Timestamptz

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimestampTzMinMaxOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_timestamptz_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Timestamptz

          @@index([data(ops: TimestampTzBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimestampTzBloomOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_timestamptz_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Timestamptz

          @@index([data(ops: TimestampTzMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimestampTzMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_time_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Time

          @@index([data(ops: TimeMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TimeMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_time_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Time

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TimeMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_time_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Time

          @@index([data(ops: TimeBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TimeBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_time_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Time

          @@index([data(ops: TimeMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimeMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_timetz_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Timetz

          @@index([data(ops: TimeTzMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TimeTzMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_timetz_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Timetz

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TimeTzMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_timetz_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Timetz

          @@index([data(ops: TimeTzBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::TimeTzBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_timetz_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int      @id @default(autoincrement())
          data DateTime @db.Timetz

          @@index([data(ops: TimeTzMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::TimeTzMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_uuid_minmax_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Uuid

          @@index([data(ops: UuidMinMaxOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::UuidMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn brin_uuid_minmax_ops_default(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Uuid

          @@index([data], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::UuidMinMaxOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_uuid_bloom_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Uuid

          @@index([data(ops: UuidBloomOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| attrs.assert_ops(SQLOperatorClassKind::UuidBloomOps))
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn brin_uuid_minmax_multi_ops(api: TestApi) {
    let dm = r#"
        model A {
          id   Int    @id @default(autoincrement())
          data String @db.Uuid

          @@index([data(ops: UuidMinMaxMultiOps)], type: Brin)
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_has_column("data")
            .assert_index_on_columns(&["data"], |idx| {
                idx.assert_algorithm(SqlIndexAlgorithm::Brin)
                    .assert_column("data", |attrs| {
                        attrs.assert_ops(SQLOperatorClassKind::UuidMinMaxMultiOps)
                    })
            })
    });

    api.schema_push_w_datasource(dm).send().assert_no_steps();
}
