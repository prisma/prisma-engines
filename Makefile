default: build

build:
	cargo build

# Build the crates with deny-warnings on to emulate CI
pedantic:
	RUSTFLAGS="-D warnings" cargo build

release:
	cargo build --release

all-dbs:
	docker-compose -f docker-compose.yml up  -d --remove-orphans mysql-5-7 postgres mysql-8-0

dev-sqlite:
	echo 'sqlite' > current_connector

dev-postgres:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres
	echo 'postgres' > current_connector

dev-mysql:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-5-7
	echo 'mysql' > current_connector

dev-mysql8:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-8-0
	echo 'mysql8' > current_connector

dev-down:
	docker-compose -f docker-compose.yml down -v --remove-orphans

use-local-migration-engine:
	cargo build --release
	cp target/release/migration-engine $(PRISMA2_BINARY_PATH)/

use-local-query-engine:
	cargo build --release
	cp target/release/prisma $(PRISMA2_BINARY_PATH)/runtime/
	cp target/release/prisma $(PRISMA2_BINARY_PATH)/query-engine-darwin
