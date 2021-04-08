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

all-dbs:
	docker-compose -f docker-compose.yml up  -d --remove-orphans

dev-sqlite:
	echo 'sqlite' > current_connector
	cp $(CONFIG_PATH)/sqlite $(CONFIG_FILE)

dev-postgres9:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres9
	echo 'postgres9' > current_connector
	cp $(CONFIG_PATH)/postgres9 $(CONFIG_FILE)

dev-postgres10:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres10
	echo 'postgres10' > current_connector
	cp $(CONFIG_PATH)/postgres10 $(CONFIG_FILE)

dev-postgres11:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres11
	echo 'postgres11' > current_connector
	cp $(CONFIG_PATH)/postgres11 $(CONFIG_FILE)

dev-postgres12:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres12
	echo 'postgres12' > current_connector
	cp $(CONFIG_PATH)/postgres12 $(CONFIG_FILE)

dev-postgres13:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres13
	echo 'postgres13' > current_connector
	cp $(CONFIG_PATH)/postgres13 $(CONFIG_FILE)

dev-pgbouncer:
	docker-compose -f docker-compose.yml up -d --remove-orphans pgbouncer postgres11
	echo 'pgbouncer' > current_connector

dev-mysql:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-5-7
	echo 'mysql' > current_connector
	cp $(CONFIG_PATH)/mysql57 $(CONFIG_FILE)

dev-mysql_5_6:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-5-6
	echo 'mysql56' > current_connector
	cp $(CONFIG_PATH)/mysql56 $(CONFIG_FILE)

dev-mysql8:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-8-0
	echo 'mysql8' > current_connector
	cp $(CONFIG_PATH)/mysql58 $(CONFIG_FILE)

dev-mariadb:
	docker-compose -f docker-compose.yml up -d --remove-orphans mariadb-10-0
	echo 'mariadb' > current_connector
	cp $(CONFIG_PATH)/mariadb $(CONFIG_FILE)

dev-mssql2019:
	docker-compose -f docker-compose.yml up -d --remove-orphans mssql-2019
	echo 'mssql2019' > current_connector
	cp $(CONFIG_PATH)/sqlserver2019 $(CONFIG_FILE)

dev-mssql2017:
	docker-compose -f docker-compose.yml up -d --remove-orphans mssql-2017
	echo 'mssql2017' > current_connector
	cp $(CONFIG_PATH)/sqlserver2017 $(CONFIG_FILE)

dev-mongodb:
	docker-compose -f docker-compose.yml up -d --remove-orphans mongo4
	echo 'mongodb' > current_connector
	cp $(CONFIG_PATH)/mongodb4 $(CONFIG_FILE)

dev-down:
	docker-compose -f docker-compose.yml down -v --remove-orphans

use-local-migration-engine:
	cargo build --release
	cp target/release/migration-engine $(PRISMA2_BINARY_PATH)/

use-local-query-engine:
	cargo build --release
	cp target/release/query-engine $(PRISMA2_BINARY_PATH)/runtime/
	cp target/release/query-engine $(PRISMA2_BINARY_PATH)/query-engine-darwin
