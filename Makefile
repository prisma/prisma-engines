REPO_ROOT := $(shell git rev-parse --show-toplevel)

CONFIG_PATH = ./query-engine/connector-test-kit-rs/test-configs
CONFIG_FILE = .test_config
SCHEMA_EXAMPLES_PATH = ./query-engine/example_schemas
DEV_SCHEMA_FILE = dev_datamodel.prisma
PRISMA_BRANCH ?= main
ENGINE_SIZE_OUTPUT ?= /dev/stdout
QE_WASM_VERSION ?= 0.0.0
SCHEMA_WASM_VERSION ?= 0.0.0

LIBRARY_EXT := $(shell                            \
    case "$$(uname -s)" in                        \
        (Darwin)               echo "dylib" ;;    \
        (MINGW*|MSYS*|CYGWIN*) echo "dll"   ;;    \
        (*)                    echo "so"    ;;    \
    esac)

PROFILE ?= dev

default: build

###############
# clean tasks #
###############

clean-qe-wasm:
	@echo "Cleaning query-engine/query-engine-wasm/pkg" && \
	cd query-engine/query-engine-wasm/pkg && find . ! -name '.' ! -name '..' ! -name 'README.md' -exec rm -rf {} +

clean-se-wasm:
	@echo "Cleaning schema-engine/schema-engine-wasm/pkg" && \
	cd schema-engine/schema-engine-wasm/pkg && find . ! -name '.' ! -name '..' ! -name 'README.md' -exec rm -rf {} +

clean-qc-wasm:
	@echo "Cleaning query-compiler/query-compiler-wasm/pkg" && \
	cd query-compiler/query-compiler-wasm/pkg && find . ! -name '.' ! -name '..' ! -name 'README.md' -exec rm -rf {} +

clean-cargo:
	@echo "Cleaning cargo" && \
	cargo clean

clean: clean-qe-wasm clean-se-wasm clean-qc-wasm clean-cargo

###################
# script wrappers #
###################

bootstrap-darwin:
	script/bootstrap-darwin

profile-shell:
	script/profile-shell

##################
# Build commands #
##################

build:
	cargo build

build-qe:
	cargo build --package query-engine

build-qe-napi:
	cargo build --package query-engine-node-api --profile $(PROFILE)

build-qe-wasm:
	cd query-engine/query-engine-wasm && \
	./build.sh $(QE_WASM_VERSION) query-engine/query-engine-wasm/pkg

build-qe-wasm-gz: build-qe-wasm
	@cd query-engine/query-engine-wasm/pkg && \
    for provider in postgresql mysql sqlite sqlserver cockroachdb; do \
        gzip -knc $$provider/query_engine_bg.wasm > $$provider.gz; \
    done;

integrate-qe-wasm:
	cd query-engine/query-engine-wasm && \
	./build.sh $(QE_WASM_VERSION) ../prisma/packages/client/node_modules/@prisma/query-engine-wasm

build-se-wasm:
	cd schema-engine/schema-engine-wasm && \
	./build.sh $(SCHEMA_ENGINE_WASM_VERSION) schema-engine/schema-engine-wasm/pkg

build-qc-wasm:
	cd query-compiler/query-compiler-wasm && \
	./build.sh $(QE_WASM_VERSION) query-compiler/query-compiler-wasm/pkg

build-qc-wasm-gz: build-qc-wasm
	@cd query-compiler/query-compiler-wasm/pkg && \
    for provider in postgresql mysql sqlite sqlserver cockroachdb; do \
        gzip -knc $$provider/query_compiler_bg.wasm > $$provider.gz; \
    done;

build-schema-wasm:
	@printf '%s\n' "🛠️  Building the Rust crate"
	cargo build --profile $(PROFILE) --target=wasm32-unknown-unknown -p prisma-schema-build

	@printf '\n%s\n' "📦 Creating the npm package"
	WASM_BUILD_PROFILE=$(PROFILE) \
	NPM_PACKAGE_VERSION=$(SCHEMA_WASM_VERSION) \
	out="$(REPO_ROOT)/target/prisma-schema-wasm" \
	./prisma-schema-wasm/scripts/install.sh

# Emulate pedantic CI compilation.
pedantic:
	cargo fmt -- --check
	cargo clippy --all-features --all-targets -- -Dwarnings
	cargo clippy --all-features --all-targets \
	    -p query-engine-wasm \
		-p schema-engine-wasm \
		-p query-compiler-wasm \
		-p prisma-schema-build \
		--target wasm32-unknown-unknown \
		-- -Dwarnings

release:
	cargo build --release

#################
# Test commands #
#################

test-qe:
ifndef DRIVER_ADAPTER
	cargo test --package query-engine-tests
else
	@echo "Executing query engine tests with $(DRIVER_ADAPTER) driver adapter"; \
	if [ "$(ENGINE)" = "wasm" ]; then \
		$(MAKE) test-driver-adapter-$(DRIVER_ADAPTER)-wasm; \
	else \
		$(MAKE) test-driver-adapter-$(DRIVER_ADAPTER); \
	fi
endif

test-qe-verbose:
	cargo test --package query-engine-tests -- --nocapture

# Single threaded thread execution.
test-qe-st:
	cargo test --package query-engine-tests -- --test-threads 1

# Single threaded thread execution, verbose.
test-qe-verbose-st:
	cargo test --package query-engine-tests -- --nocapture --test-threads 1

# Black-box tests, exercising the query engine HTTP apis
test-qe-black-box: build-qe
	cargo test --package black-box-tests -- --test-threads 1

test-unit:
	cargo test --workspace --all-features \
	    --exclude=quaint \
	    --exclude=query-engine \
	    --exclude=query-engine-node-api \
	    --exclude=black-box-tests \
	    --exclude=query-engine-tests \
	    --exclude=sql-migration-tests \
	    --exclude=schema-engine-cli \
	    --exclude=sql-schema-describer \
	    --exclude=sql-introspection-tests \
	    --exclude=mongodb-schema-connector

check-schema-wasm-package: build-schema-wasm
	PRISMA_SCHEMA_WASM="$(REPO_ROOT)/target/prisma-schema-wasm" \
	out=$(shell mktemp -d) \
	NODE=$(shell which node) \
	./prisma-schema-wasm/scripts/check.sh

###########################
# Database setup commands #
###########################

all-dbs-up:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans

all-dbs-down:
	docker compose -f docker-compose.yml down -v --remove-orphans

start-sqlite:

dev-sqlite:
	cp $(CONFIG_PATH)/sqlite $(CONFIG_FILE)

dev-react-native:
	cp $(CONFIG_PATH)/react-native $(CONFIG_FILE)

dev-libsql-wasm: build-qe-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/libsql-wasm $(CONFIG_FILE)

test-libsql-wasm: dev-libsql-wasm test-qe-st
test-driver-adapter-libsql-wasm: test-libsql-wasm

dev-libsql-qc: build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/libsql-qc $(CONFIG_FILE)

test-libsql-qc: dev-libsql-qc test-qe-st

dev-better-sqlite3-qc: build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/better-sqlite3-qc $(CONFIG_FILE)

test-better-sqlite3-qc: dev-better-sqlite3-qc test-qe-st

dev-d1-qc: build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/d1-qc $(CONFIG_FILE)

test-d1-qc: dev-d1-qc test-qe-st

dev-d1: build-qe-wasm build-se-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/cloudflare-d1 $(CONFIG_FILE)

test-d1: dev-d1 test-qe-st
test-driver-adapter-d1: test-d1

dev-better-sqlite3-wasm: build-qe-wasm build-se-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/better-sqlite3-wasm $(CONFIG_FILE)

test-better-sqlite3-wasm: dev-better-sqlite3-wasm test-qe-st
test-driver-adapter-better-sqlite3-wasm: test-better-sqlite3-wasm

start-postgres12:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres12

dev-postgres12: start-postgres12
	cp $(CONFIG_PATH)/postgres12 $(CONFIG_FILE)

start-postgres13:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres13

dev-postgres13: start-postgres13
	cp $(CONFIG_PATH)/postgres13 $(CONFIG_FILE)

dev-pg-wasm: start-postgres13 build-qe-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/pg-wasm $(CONFIG_FILE)

test-pg-wasm: dev-pg-wasm test-qe-st

dev-pg-qc: start-postgres13 build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/pg-qc $(CONFIG_FILE)

dev-pg-qc-join:
	PRISMA_RELATION_LOAD_STRATEGY=join make dev-pg-qc

dev-pg-qc-query:
	PRISMA_RELATION_LOAD_STRATEGY=query make dev-pg-qc

test-pg-qc: dev-pg-qc test-qe

test-pg-qc-join:
	PRISMA_RELATION_LOAD_STRATEGY=join make test-pg-qc

test-pg-qc-query:
	PRISMA_RELATION_LOAD_STRATEGY=query make test-pg-qc

test-driver-adapter-pg: test-pg-js
test-driver-adapter-pg-wasm: test-pg-wasm

start-pg-bench:
	docker compose -f libs/driver-adapters/executor/bench/docker-compose.yml up --wait -d --remove-orphans postgres

setup-pg-bench: start-pg-bench build-qe-napi build-qe-wasm build-driver-adapters-kit-qe

dev-pg-cockroachdb-wasm: start-cockroach_23_1 build-qe-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/pg-cockroachdb-wasm $(CONFIG_FILE)

test-pg-cockroachdb-wasm: dev-pg-cockroachdb-wasm test-qe-st

dev-pg-cockroachdb-qc: start-cockroach_23_1 build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/pg-cockroachdb-qc $(CONFIG_FILE)

dev-pg-cockroachdb-qc-join:
	PRISMA_RELATION_LOAD_STRATEGY=join make dev-pg-cockroachdb-qc

dev-pg-cockroachdb-qc-query:
	PRISMA_RELATION_LOAD_STRATEGY=query make dev-pg-cockroachdb-qc

test-pg-cockroachdb-qc: dev-pg-cockroachdb-qc test-qe

test-pg-cockroachdb-qc-join:
	PRISMA_RELATION_LOAD_STRATEGY=join make test-pg-cockroachdb-qc

test-pg-cockroachdb-qc-query:
	PRISMA_RELATION_LOAD_STRATEGY=query make test-pg-cockroachdb-qc

run-bench:
	DATABASE_URL="postgresql://postgres:postgres@localhost:5432/bench?schema=imdb_bench&sslmode=disable" \
	node --experimental-wasm-modules libs/driver-adapters/executor/dist/bench.mjs

bench-pg-js: setup-pg-bench run-bench

start-neon:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans neon-proxy

dev-neon-wasm: start-neon build-qe-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/neon-wasm $(CONFIG_FILE)

test-neon-wasm: dev-neon-wasm test-qe-st

dev-neon-qc: start-neon build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/neon-qc $(CONFIG_FILE)

dev-neon-qc-join:
	PRISMA_RELATION_LOAD_STRATEGY=join make dev-neon-qc

dev-neon-qc-query:
	PRISMA_RELATION_LOAD_STRATEGY=query make dev-neon-qc

test-neon-qc: dev-neon-qc test-qe

test-neon-qc-join:
	PRISMA_RELATION_LOAD_STRATEGY=join make test-neon-qc

test-neon-qc-query:
	PRISMA_RELATION_LOAD_STRATEGY=query make test-neon-qc

test-driver-adapter-neon: test-neon-js
test-driver-adapter-neon-wasm: test-neon-wasm

start-postgres14:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres14

dev-postgres14: start-postgres14
	cp $(CONFIG_PATH)/postgres14 $(CONFIG_FILE)

start-postgres15:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans postgres15

dev-postgres15: start-postgres15
	cp $(CONFIG_PATH)/postgres15 $(CONFIG_FILE)

start-postgres16:
	docker compose -f docker-compose.yml up -d --remove-orphans postgres16

dev-postgres16: start-postgres16
	cp $(CONFIG_PATH)/postgres16 $(CONFIG_FILE)

start-cockroach_23_1:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans cockroach_23_1

dev-cockroach_23_1: start-cockroach_23_1
	cp $(CONFIG_PATH)/cockroach_23_1 $(CONFIG_FILE)

start-cockroach_22_2:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans cockroach_22_2

dev-cockroach_22_2: start-cockroach_22_2
	cp $(CONFIG_PATH)/cockroach_22_2 $(CONFIG_FILE)

start-cockroach_22_1_0:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans cockroach_22_1_0

dev-cockroach_22_1_0: start-cockroach_22_1_0
	cp $(CONFIG_PATH)/cockroach_22_1 $(CONFIG_FILE)

start-cockroach_21_2_0_patched:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans cockroach_21_2_0_patched

dev-cockroach_21_2_0_patched: start-cockroach_21_2_0_patched
	cp $(CONFIG_PATH)/cockroach_21_2_0_patched $(CONFIG_FILE)

dev-pgbouncer:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans pgbouncer postgres11

start-mysql_5_7:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mysql-5-7

dev-mysql: start-mysql_5_7
	cp $(CONFIG_PATH)/mysql57 $(CONFIG_FILE)

start-mysql_5_6:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mysql-5-6

dev-mysql_5_6: start-mysql_5_6
	cp $(CONFIG_PATH)/mysql56 $(CONFIG_FILE)

start-mysql_8:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mysql-8-0

dev-mysql8: start-mysql_8
	cp $(CONFIG_PATH)/mysql8 $(CONFIG_FILE)

start-mysql_mariadb:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mariadb-10-0

start-mysql_mariadb_11:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mariadb-11-0

dev-mariadb: start-mysql_mariadb
	cp $(CONFIG_PATH)/mariadb $(CONFIG_FILE)

dev-mariadb11: start-mysql_mariadb_11
	cp $(CONFIG_PATH)/mariadb $(CONFIG_FILE)

start-mssql_2019:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mssql-2019

dev-mssql2019: start-mssql_2019
	cp $(CONFIG_PATH)/sqlserver2019 $(CONFIG_FILE)

start-mssql_2022:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mssql-2022

dev-mssql2022: start-mssql_2022
	cp $(CONFIG_PATH)/sqlserver2022 $(CONFIG_FILE)

start-mssql_edge:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans azure-edge

dev-mssql_edge: start-mssql_edge
	cp $(CONFIG_PATH)/sqlserver2019 $(CONFIG_FILE)

dev-mssql-wasm: start-mssql_2022 build-qe-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/sqlserver-wasm $(CONFIG_FILE)

dev-mssql-qc: start-mssql_2022 build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/sqlserver-qc $(CONFIG_FILE)

test-mssql-qc: dev-mssql-qc test-qe

start-mssql_2017:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mssql-2017

dev-mssql2017: start-mssql_2017
	cp $(CONFIG_PATH)/sqlserver2017 $(CONFIG_FILE)

start-mongodb42-single:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo42-single

start-mongodb44-single:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo44-single

start-mongodb4-single: start-mongodb44-single

start-mongodb5-single:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo5-single

start-mongodb_4_2:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo42

start-mongodb_4_4:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo44

dev-mongodb_4_4: start-mongodb_4_4
	cp $(CONFIG_PATH)/mongodb44 $(CONFIG_FILE)

start-mongodb_5:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans mongo5

dev-mongodb_5: start-mongodb_5
	cp $(CONFIG_PATH)/mongodb5 $(CONFIG_FILE)

dev-mongodb_5_single: start-mongodb5-single
	cp $(CONFIG_PATH)/mongodb5 $(CONFIG_FILE)

dev-mongodb_4_2: start-mongodb_4_2
	cp $(CONFIG_PATH)/mongodb42 $(CONFIG_FILE)

start-vitess_8_0:
	docker compose -f docker-compose.yml up --wait -d --remove-orphans vitess-test-8_0 vitess-shadow-8_0

dev-vitess_8_0: start-vitess_8_0
	cp $(CONFIG_PATH)/vitess_8_0 $(CONFIG_FILE)

start-planetscale:
	docker compose -f docker-compose.yml up -d --remove-orphans planetscale-proxy

dev-planetscale-wasm: start-planetscale build-qe-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/planetscale-wasm $(CONFIG_FILE)

test-planetscale-wasm: dev-planetscale-wasm test-qe-st

dev-planetscale-qc: start-planetscale build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/planetscale-qc $(CONFIG_FILE)

test-planetscale-qc: dev-planetscale-qc test-qe-st

test-driver-adapter-planetscale: test-planetscale-js
test-driver-adapter-planetscale-wasm: test-planetscale-wasm

dev-mariadb-mysql-wasm: start-mysql_8 build-qe-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/mariadb-mysql-wasm $(CONFIG_FILE)

test-mariadb-mysql-wasm: dev-mariadb-mysql-wasm test-qe-st

dev-mariadb-mysql-qc: start-mysql_8 build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/mariadb-mysql-qc $(CONFIG_FILE)

test-mariadb-mysql-qc: dev-mariadb-mysql-qc test-qe-st

test-driver-adapter-mariadb-mysql: test-mariadb-mysql-js
test-driver-adapter-mariadb-mysql-wasm: test-mariadb-mysql-wasm

dev-mariadb-wasm: start-mysql_mariadb build-qe-wasm build-driver-adapters-kit-qe
	cp $(CONFIG_PATH)/mariadb-wasm $(CONFIG_FILE)

test-mariadb-wasm: dev-mariadb-wasm test-qe-st

dev-mariadb-qc: start-mysql_mariadb build-qc-wasm build-driver-adapters-kit-qc
	cp $(CONFIG_PATH)/mariadb-qc $(CONFIG_FILE)

test-mariadb-qc: dev-mariadb-qc test-qe-st

test-driver-adapter-mariadb: test-mariadb-js
test-driver-adapter-mariadb-wasm: test-mariadb-wasm

######################
# Local dev commands #
######################

measure-qe-wasm: build-qe-wasm-gz
	@cd query-engine/query-engine-wasm/pkg; \
	for provider in postgresql mysql sqlite sqlserver cockroachdb; do \
		echo "$${provider}_qe_size=$$(cat $$provider/query_engine_bg.wasm | wc -c | tr -d ' ')" >> $(ENGINE_SIZE_OUTPUT); \
		echo "$${provider}_qe_size_gz=$$(cat $$provider.gz | wc -c | tr -d ' ')" >> $(ENGINE_SIZE_OUTPUT); \
	done;

measure-qc-wasm: build-qc-wasm-gz
	@cd query-compiler/query-compiler-wasm/pkg; \
	for provider in postgresql mysql sqlite sqlserver cockroachdb; do \
		echo "$${provider}_qc_size=$$(cat $$provider/query_compiler_bg.wasm | wc -c | tr -d ' ')" >> $(ENGINE_SIZE_OUTPUT); \
		echo "$${provider}_qc_size_gz=$$(cat $$provider.gz | wc -c | tr -d ' ')" >> $(ENGINE_SIZE_OUTPUT); \
	done;

install-driver-adapters-kit-deps: build-driver-adapters
	cd libs/driver-adapters && pnpm i

build-driver-adapters-kit: install-driver-adapters-kit-deps
	cd libs/driver-adapters && pnpm build

build-driver-adapters-kit-qe: install-driver-adapters-kit-deps
	cd libs/driver-adapters && pnpm build:qe

build-driver-adapters-kit-qc: install-driver-adapters-kit-deps
	cd libs/driver-adapters && pnpm build:qc

build-driver-adapters: ensure-prisma-present
	@echo "Building driver adapters..."
	@cd ../prisma && pnpm i
	@echo "Driver adapters build completed.";

ensure-prisma-present:
	@if [ -d ../prisma ]; then \
		cd "$(realpath ../prisma)" && git fetch origin main; \
		LOCAL_CHANGES=$$(git diff --name-only HEAD origin/main -- 'packages/*adapter*'); \
		if [ -n "$$LOCAL_CHANGES" ]; then \
		  echo "⚠️ ../prisma diverges from prisma/prisma main branch. Test results might diverge from those in CI ⚠️ "; \
		fi \
	else \
		echo "git clone --depth=1 https://github.com/prisma/prisma.git --branch=$(PRISMA_BRANCH) ../prisma"; \
		git clone --depth=1 https://github.com/prisma/prisma.git --branch=$(PRISMA_BRANCH) "../prisma" && echo "Prisma repository has been cloned to ../prisma"; \
	fi;

qe:
	cargo run --bin query-engine -- --engine-protocol json --enable-raw-queries --enable-open-telemetry --enable-telemetry-in-response

qe-graphql:
	cargo run --bin query-engine -- --engine-protocol graphql --enable-playground --enable-raw-queries --enable-open-telemetry --enable-telemetry-in-response

qe-dmmf:
	cargo run --bin query-engine -- cli dmmf > dmmf.json

qe-dev-mongo_4_4: start-mongodb_4_4
	cp $(SCHEMA_EXAMPLES_PATH)/generic_mongo4.prisma $(DEV_SCHEMA_FILE)

## OpenTelemetry
otel:
	docker compose up --remove-orphans -d otel

# Build the debug version of Query Engine Node-API library ready to be consumed by Node.js
.PHONY: qe-node-api
qe-node-api: build target/debug/libquery_engine.node --profile=$(PROFILE)

%.node: %.$(LIBRARY_EXT)
# Remove the file first to work around a macOS bug: https://openradar.appspot.com/FB8914243
# otherwise macOS gatekeeper may kill the Node.js process when it tries to load the library
	if [[ "$$(uname -sm)" == "Darwin arm64" ]]; then rm -f $@; fi
	cp $< $@
