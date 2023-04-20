//! https://www.notion.so/prismaio/PostgreSQL-Exclusion-Constraints-fb2ecc44f773463f908d3d0e2d737271

use indoc::indoc;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Postgres))]
async fn aragon_test_postgres(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
      CREATE TABLE public.eventsources (
        eventsource_id integer NOT NULL,
        created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP,
        enabled boolean DEFAULT true NOT NULL,
        contract_address character varying(255) NOT NULL,
        kernel_address character varying(255) NOT NULL,
        ens_name character varying(255),
        abi jsonb,
        event_name character varying(255) NOT NULL,
        app_name character varying(255) NOT NULL,
        network character varying(255) NOT NULL,
        from_block bigint NOT NULL,
        last_poll timestamp with time zone
      );

      CREATE TABLE public.knex_migrations (
        id integer NOT NULL,
        name character varying(255),
        batch integer,
        migration_time timestamp with time zone
      );

      CREATE TABLE public.knex_migrations_lock (
        index integer NOT NULL,
        is_locked integer
      );

      CREATE TABLE public.notifications (
        notification_id integer NOT NULL,
        created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP,
        subscription_id integer NOT NULL,
        return_values jsonb,
        block_time timestamp with time zone NOT NULL,
        transaction_hash character varying(255) NOT NULL,
        block bigint NOT NULL,
        sent boolean DEFAULT false NOT NULL
      );

      CREATE TABLE public.subscriptions (
        subscription_id integer NOT NULL,
        user_id integer NOT NULL,
        eventsource_id integer NOT NULL,
        created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP,
        join_block bigint NOT NULL
      );

      CREATE TABLE public.tokens (
        token_id integer NOT NULL,
        created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP,
        user_id integer NOT NULL,
        token_scope text DEFAULT 'MAGICLINK'::text,
        valid boolean DEFAULT true NOT NULL,
        some_new_field character varying(255),
        CONSTRAINT tokens_token_scope_check CHECK ((token_scope = ANY (ARRAY['MAGICLINK'::text, 'API'::text])))
      );

      CREATE TABLE public.users (
        user_id integer NOT NULL,
        created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP,
        email character varying(255) NOT NULL,
        verified boolean DEFAULT false NOT NULL
      );

      ALTER TABLE ONLY public.eventsources
        ADD CONSTRAINT eventsources_contract_address_event_name_network_unique UNIQUE (contract_address, event_name, network);

      ALTER TABLE ONLY public.eventsources
        ADD CONSTRAINT eventsources_pkey PRIMARY KEY (eventsource_id);

      ALTER TABLE ONLY public.knex_migrations_lock
        ADD CONSTRAINT knex_migrations_lock_pkey PRIMARY KEY (index);

      ALTER TABLE ONLY public.knex_migrations
        ADD CONSTRAINT knex_migrations_pkey PRIMARY KEY (id);

      ALTER TABLE ONLY public.notifications
        ADD CONSTRAINT notifications_pkey PRIMARY KEY (notification_id);

      ALTER TABLE ONLY public.subscriptions
        ADD CONSTRAINT subscriptions_pkey PRIMARY KEY (subscription_id);

      ALTER TABLE ONLY public.subscriptions
        ADD CONSTRAINT subscriptions_user_id_eventsource_id_unique UNIQUE (user_id, eventsource_id);

      ALTER TABLE ONLY public.tokens
        ADD CONSTRAINT tokens_pkey PRIMARY KEY (token_id);

      ALTER TABLE ONLY public.users
        ADD CONSTRAINT users_email_unique UNIQUE (email);

      ALTER TABLE ONLY public.users
        ADD CONSTRAINT users_pkey PRIMARY KEY (user_id);

      CREATE INDEX notifications_subscription_id_index ON public.notifications USING btree (subscription_id);

      CREATE INDEX subscriptions_eventsource_id_index ON public.subscriptions USING btree (eventsource_id);

      CREATE INDEX subscriptions_user_id_index ON public.subscriptions USING btree (user_id);

      CREATE INDEX tokens_user_id_index ON public.tokens USING btree (user_id);

      CREATE INDEX users_email_index ON public.users USING btree (email);

      ALTER TABLE ONLY public.notifications
        ADD CONSTRAINT notifications_subscription_id_foreign FOREIGN KEY (subscription_id) REFERENCES public.subscriptions(subscription_id) ON DELETE CASCADE;

      ALTER TABLE ONLY public.subscriptions
        ADD CONSTRAINT subscriptions_eventsource_id_foreign FOREIGN KEY (eventsource_id) REFERENCES public.eventsources(eventsource_id);

      ALTER TABLE ONLY public.subscriptions
        ADD CONSTRAINT subscriptions_user_id_foreign FOREIGN KEY (user_id) REFERENCES public.users(user_id) ON DELETE CASCADE;

      ALTER TABLE ONLY public.tokens
        ADD CONSTRAINT tokens_user_id_foreign FOREIGN KEY (user_id) REFERENCES public.users(user_id) ON DELETE CASCADE;
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect!["[]"];

    api.expect_warnings(&expectation).await;

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn aragon_test_cockroachdb(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
      CREATE TABLE public._prisma_migrations (
        id VARCHAR(36) NOT NULL,
        checksum VARCHAR(64) NOT NULL,
        finished_at TIMESTAMPTZ NULL,
        migration_name VARCHAR(255) NOT NULL,
        logs STRING NULL,
        rolled_back_at TIMESTAMPTZ NULL,
        started_at TIMESTAMPTZ NOT NULL DEFAULT now():::TIMESTAMPTZ,
        applied_steps_count INT8 NOT NULL DEFAULT 0:::INT8,
        CONSTRAINT "primary" PRIMARY KEY (id ASC)
      );
      
      CREATE TABLE public.eventsources (
        eventsource_id INT8 NOT NULL,
        created_at TIMESTAMPTZ NULL DEFAULT current_timestamp():::TIMESTAMPTZ,
        enabled BOOL NOT NULL DEFAULT true,
        contract_address VARCHAR(255) NOT NULL,
        kernel_address VARCHAR(255) NOT NULL,
        ens_name VARCHAR(255) NULL,
        abi JSONB NULL,
        event_name VARCHAR(255) NOT NULL,
        app_name VARCHAR(255) NOT NULL,
        network VARCHAR(255) NOT NULL,
        from_block INT8 NOT NULL,
        last_poll TIMESTAMPTZ NULL,
        rowid INT8 NOT VISIBLE NOT NULL DEFAULT unique_rowid(),
        CONSTRAINT eventsources_pkey PRIMARY KEY (eventsource_id ASC),
        UNIQUE INDEX eventsources_contract_address_event_name_network_unique (contract_address ASC, event_name ASC, network ASC)
      );
      
      CREATE TABLE public.knex_migrations (
        id INT8 NOT NULL,
        name VARCHAR(255) NULL,
        batch INT8 NULL,
        migration_time TIMESTAMPTZ NULL,
        rowid INT8 NOT VISIBLE NOT NULL DEFAULT unique_rowid(),
        CONSTRAINT knex_migrations_pkey PRIMARY KEY (id ASC)
      );
      
      CREATE TABLE public.users (
        user_id INT8 NOT NULL,
        created_at TIMESTAMPTZ NULL DEFAULT current_timestamp():::TIMESTAMPTZ,
        email VARCHAR(255) NOT NULL,
        verified BOOL NOT NULL DEFAULT false,
        rowid INT8 NOT VISIBLE NOT NULL DEFAULT unique_rowid(),
        CONSTRAINT users_pkey PRIMARY KEY (user_id ASC),
        UNIQUE INDEX users_email_unique (email ASC),
        INDEX users_email_index (email ASC)
      );
      
      CREATE TABLE public.subscriptions (
        subscription_id INT8 NOT NULL,
        user_id INT8 NOT NULL,
        eventsource_id INT8 NOT NULL,
        created_at TIMESTAMPTZ NULL DEFAULT current_timestamp():::TIMESTAMPTZ,
        join_block INT8 NOT NULL,
        rowid INT8 NOT VISIBLE NOT NULL DEFAULT unique_rowid(),
        CONSTRAINT subscriptions_pkey PRIMARY KEY (subscription_id ASC),
        UNIQUE INDEX subscriptions_user_id_eventsource_id_unique (user_id ASC, eventsource_id ASC),
        INDEX subscriptions_eventsource_id_index (eventsource_id ASC),
        INDEX subscriptions_user_id_index (user_id ASC)
      );
      
      CREATE TABLE public.notifications (
        notification_id INT8 NOT NULL,
        created_at TIMESTAMPTZ NULL DEFAULT current_timestamp():::TIMESTAMPTZ,
        subscription_id INT8 NOT NULL,
        return_values JSONB NULL,
        block_time TIMESTAMPTZ NOT NULL,
        transaction_hash VARCHAR(255) NOT NULL,
        block INT8 NOT NULL,
        sent BOOL NOT NULL DEFAULT false,
        rowid INT8 NOT VISIBLE NOT NULL DEFAULT unique_rowid(),
        CONSTRAINT notifications_pkey PRIMARY KEY (notification_id ASC),
        INDEX notifications_subscription_id_index (subscription_id ASC)
      );
      
      CREATE TABLE public.tokens (
        token_id INT8 NOT NULL,
        created_at TIMESTAMPTZ NULL DEFAULT current_timestamp():::TIMESTAMPTZ,
        user_id INT8 NOT NULL,
        token_scope STRING NULL DEFAULT 'MAGICLINK':::STRING,
        valid BOOL NOT NULL DEFAULT true,
        some_new_field VARCHAR(255) NULL,
        rowid INT8 NOT VISIBLE NOT NULL DEFAULT unique_rowid(),
        CONSTRAINT tokens_pkey PRIMARY KEY (token_id ASC),
        INDEX tokens_user_id_index (user_id ASC),
        CONSTRAINT tokens_token_scope_check CHECK (token_scope = ANY ARRAY['MAGICLINK':::STRING, 'API':::STRING]:::STRING[])
      );
      
      ALTER TABLE public.subscriptions ADD CONSTRAINT subscriptions_eventsource_id_foreign FOREIGN KEY (eventsource_id) REFERENCES public.eventsources(eventsource_id);
      ALTER TABLE public.subscriptions ADD CONSTRAINT subscriptions_user_id_foreign FOREIGN KEY (user_id) REFERENCES public.users(user_id) ON DELETE CASCADE;
      ALTER TABLE public.notifications ADD CONSTRAINT notifications_subscription_id_foreign FOREIGN KEY (subscription_id) REFERENCES public.subscriptions(subscription_id) ON DELETE CASCADE;
      ALTER TABLE public.tokens ADD CONSTRAINT tokens_user_id_foreign FOREIGN KEY (user_id) REFERENCES public.users(user_id) ON DELETE CASCADE;
      ALTER TABLE public.subscriptions VALIDATE CONSTRAINT subscriptions_eventsource_id_foreign;
      ALTER TABLE public.subscriptions VALIDATE CONSTRAINT subscriptions_user_id_foreign;
      ALTER TABLE public.notifications VALIDATE CONSTRAINT notifications_subscription_id_foreign;
      ALTER TABLE public.tokens VALIDATE CONSTRAINT tokens_user_id_foreign;
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "cockroachdb"
          url      = "env(TEST_DATABASE_URL)"
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect!["[]"];

    api.expect_warnings(&expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres, CockroachDb))]
async fn check_and_exclusion_constraints_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE EXTENSION btree_gist;
    
        CREATE TABLE room_reservation (
            room_reservation_id serial PRIMARY KEY,
            room_id integer NOT NULL, -- this could e.g. be a foreign key to a `room` table
            reserved_at timestamptz NOT NULL,
            reserved_until timestamptz NOT NULL,
            canceled boolean DEFAULT false,
            price numeric CHECK (price > 0),
            EXCLUDE USING gist (
                room_id WITH =, tstzrange(reserved_at, reserved_until) WITH &&
            ) WHERE (NOT canceled)
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
          price               Decimal? @db.Decimal
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        [
          {
            "code": 33,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_price_check"
              }
            ]
          },
          {
            "code": 34,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_room_id_tstzrange_excl"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
          price               Decimal? @db.Decimal
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
          price               Decimal? @db.Decimal
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn exclusion_constraints_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE EXTENSION btree_gist;
  
        CREATE TABLE room_reservation (
            room_reservation_id serial PRIMARY KEY,
            room_id integer NOT NULL, -- this could e.g. be a foreign key to a `room` table
            reserved_at timestamptz NOT NULL,
            reserved_until timestamptz NOT NULL,
            canceled boolean DEFAULT false,
            EXCLUDE USING gist (
                room_id WITH =, tstzrange(reserved_at, reserved_until) WITH &&
            ) WHERE (NOT canceled)
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        [
          {
            "code": 34,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_room_id_tstzrange_excl"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
          canceled            Boolean? @default(false)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn exclusion_constraints_without_where_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE EXTENSION btree_gist;
  
        CREATE TABLE room_reservation (
            room_reservation_id serial PRIMARY KEY,
            room_id integer NOT NULL, -- this could e.g. be a foreign key to a `room` table
            reserved_at timestamptz NOT NULL,
            reserved_until timestamptz NOT NULL,
            EXCLUDE USING gist (
                room_id WITH =, tstzrange(reserved_at, reserved_until) WITH &&
            )
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        [
          {
            "code": 34,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_room_id_tstzrange_excl"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int      @id @default(autoincrement())
          room_id             Int
          reserved_at         DateTime @db.Timestamptz(6)
          reserved_until      DateTime @db.Timestamptz(6)
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn exclusion_constraints_without_where_and_expressions_stopgap(api: &mut TestApi) -> TestResult {
    let raw_sql = indoc! {r#"
        CREATE EXTENSION btree_gist;
    
        CREATE TABLE room_reservation (
            room_reservation_id serial PRIMARY KEY,
            room_id integer NOT NULL, -- this could e.g. be a foreign key to a `room` table
            EXCLUDE USING gist (
                room_id WITH =
            )
        );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "postgresql"
          url      = "env(TEST_DATABASE_URL)"
        }

        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int @id @default(autoincrement())
          room_id             Int
        }
    "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
        [
          {
            "code": 34,
            "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support exclusion constraints. Read more: https://pris.ly/d/postgres-exclusion-constraints",
            "affected": [
              {
                "model": "room_reservation",
                "constraint": "room_reservation_room_id_excl"
              }
            ]
          }
        ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int @id @default(autoincrement())
          room_id             Int
        }
    "#};

    let expectation = expect![[r#"
        /// This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-exclusion-constraints for more info.
        model room_reservation {
          room_reservation_id Int @id @default(autoincrement())
          room_id             Int
        }
    "#]];

    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn check_constraints_stopgap(api: &mut TestApi) -> TestResult {
    // https://www.notion.so/prismaio/Indexes-Constraints-Check-constraints-PostgreSQL-cde0bee25f6343d8bbd0f7e84932e808
    let raw_sql = indoc! {r#"
      CREATE TABLE products (
          product_id serial PRIMARY KEY,
          name text,
          price numeric CHECK (price > 0)
      );
    "#};

    api.raw_cmd(raw_sql).await;

    let schema = expect![[r#"
          generator client {
            provider = "prisma-client-js"
          }

          datasource db {
            provider = "postgresql"
            url      = "env(TEST_DATABASE_URL)"
          }

          /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
          model products {
            product_id Int      @id @default(autoincrement())
            name       String?
            price      Decimal? @db.Decimal
          }
      "#]];

    api.expect_datamodel(&schema).await;

    // ensure the introspected schema is valid
    psl::parse_schema(schema.data()).unwrap();

    let expectation = expect![[r#"
          [
            {
              "code": 33,
              "message": "These constraints are not supported by the Prisma Client, because Prisma currently does not fully support check constraints. Read more: https://pris.ly/d/postgres-check-constraints",
              "affected": [
                {
                  "model": "products",
                  "constraint": "products_price_check"
                }
              ]
            }
          ]"#]];

    api.expect_warnings(&expectation).await;

    let input = indoc! { r#"
        /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
        model products {
          product_id Int      @id @default(autoincrement())
          name       String?
          price      Decimal? @db.Decimal
        }
      "#
    };

    let expectation = expect![[r#"
          /// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/postgres-check-constraints for more info.
          model products {
            product_id Int      @id @default(autoincrement())
            name       String?
            price      Decimal? @db.Decimal
          }
      "#]];
    api.expect_re_introspected_datamodel(input, expectation).await;

    Ok(())
}
