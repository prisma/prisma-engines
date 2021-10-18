CONFIG_PATH = ./query-engine/connector-test-kit-rs/test-configs
CONFIG_FILE = .test_config


default: build

build:
	cargo build

# Emulate pedantic CI compilation.
pedantic:
	RUSTFLAGS="-D warnings" cargo fmt -- --check && RUSTFLAGS="-D warnings" cargo clippy --all-targets

release:
	cargo build --release

test-qe:
	cargo test --package query-engine-tests

test-qe-verbose:
	cargo test --package query-engine-tests -- --nocapture

# Single threaded thread execution.
test-qe-st:
	cargo test --package query-engine-tests -- --test-threads 1

# Single threaded thread execution, verbose.
test-qe-verbose-st:
	cargo test --package query-engine-tests -- --nocapture --test-threads 1

all-dbs:
	docker-compose -f docker-compose.yml up  -d --remove-orphans

start-sqlite:

dev-sqlite:
	echo 'sqlite' > current_connector
	cp $(CONFIG_PATH)/sqlite $(CONFIG_FILE)

start-postgres9:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres9

dev-postgres9: start-postgres9
	echo 'postgres9' > current_connector
	cp $(CONFIG_PATH)/postgres9 $(CONFIG_FILE)

start-postgres10:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres10

dev-postgres10: start-postgres10
	echo 'postgres10' > current_connector
	cp $(CONFIG_PATH)/postgres10 $(CONFIG_FILE)

start-postgres11:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres11

dev-postgres11: start-postgres11
	echo 'postgres11' > current_connector
	cp $(CONFIG_PATH)/postgres11 $(CONFIG_FILE)

start-postgres12:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres12

dev-postgres12: start-postgres12
	echo 'postgres12' > current_connector
	cp $(CONFIG_PATH)/postgres12 $(CONFIG_FILE)

start-postgres13:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres13

dev-postgres13: start-postgres13
	echo 'postgres13' > current_connector
	cp $(CONFIG_PATH)/postgres13 $(CONFIG_FILE)

start-cockroach:
	docker-compose -f docker-compose.yml up -d --remove-orphans cockroach
	docker exec -d prisma-engines_cockroach_1 cockroach sql --insecure -e "set cluster setting sql.defaults.default_int_size = 4;"
	docker exec -d prisma-engines_cockroach_1 cockroach sql --insecure -e "set cluster setting sql.defaults.serial_normalization = 'sql_sequence_cached';"

dev-cockroach: start-cockroach
	echo 'cockroach' > current_connector
	cp $(CONFIG_PATH)/cockroach $(CONFIG_FILE)

dev-pgbouncer:
	docker-compose -f docker-compose.yml up -d --remove-orphans pgbouncer postgres11
	echo 'pgbouncer' > current_connector

start-mysql_5_7:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-5-7

dev-mysql: start-mysql_5_7
	echo 'mysql' > current_connector
	cp $(CONFIG_PATH)/mysql57 $(CONFIG_FILE)

start-mysql_5_6:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-5-6

dev-mysql_5_6: start-mysql_5_6
	echo 'mysql56' > current_connector
	cp $(CONFIG_PATH)/mysql56 $(CONFIG_FILE)

start-mysql_8:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-8-0

dev-mysql8: start-mysql_8
	echo 'mysql8' > current_connector
	cp $(CONFIG_PATH)/mysql8 $(CONFIG_FILE)

start-mysql_mariadb:
	docker-compose -f docker-compose.yml up -d --remove-orphans mariadb-10-0

dev-mariadb: start-mysql_mariadb
	echo 'mariadb' > current_connector
	cp $(CONFIG_PATH)/mariadb $(CONFIG_FILE)

start-mssql_2019:
	docker-compose -f docker-compose.yml up -d --remove-orphans mssql-2019

dev-mssql2019: start-mssql_2019
	echo 'mssql2019' > current_connector
	cp $(CONFIG_PATH)/sqlserver2019 $(CONFIG_FILE)

start-mssql_2017:
	docker-compose -f docker-compose.yml up -d --remove-orphans mssql-2017

dev-mssql2017: start-mssql_2017
	echo 'mssql2017' > current_connector
	cp $(CONFIG_PATH)/sqlserver2017 $(CONFIG_FILE)

start-mongodb:
	docker-compose -f docker-compose.yml up -d --remove-orphans mongo4

start-mongodb4-single:
	docker-compose -f docker-compose.yml up -d --remove-orphans mongo4-single

start-mongodb5-single:
	docker-compose -f docker-compose.yml up -d --remove-orphans mongo5-single

dev-mongodb: start-mongodb
	echo 'mongodb' > current_connector
	cp $(CONFIG_PATH)/mongodb4 $(CONFIG_FILE)

start-vitess_5_7:
	docker-compose -f docker-compose.yml up -d --remove-orphans vitess-test-5_7 vitess-shadow-5_7

dev-vitess_5_7: start-vitess_5_7
	echo 'vitess_5_7' > current_connector
	cp $(CONFIG_PATH)/vitess_5_7 $(CONFIG_FILE)

start-vitess_8_0:
	docker-compose -f docker-compose.yml up -d --remove-orphans vitess-test-8_0 vitess-shadow-8_0

dev-vitess_8_0: start-vitess_8_0
	echo 'vitess_8_0' > current_connector
	cp $(CONFIG_PATH)/vitess_8_0 $(CONFIG_FILE)

dev-down:
	docker-compose -f docker-compose.yml down -v --remove-orphans

use-local-migration-engine:
	cargo build --release
	cp target/release/migration-engine $(PRISMA2_BINARY_PATH)/

use-local-query-engine:
	cargo build --release
	cp target/release/query-engine $(PRISMA2_BINARY_PATH)/runtime/
	cp target/release/query-engine $(PRISMA2_BINARY_PATH)/query-engine-darwin
