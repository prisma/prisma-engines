default: build

build:
	cargo build

# Build the crates with deny-warnings on to emulate CI
pedantic:
	RUSTFLAGS="-D warnings" cargo build

release:
	cargo build --release

all-dbs:
	docker-compose -f docker-compose.yml up  -d --remove-orphans mysql-5-7 mysql-8-0 mariadb-10-0 postgres9 postgres10 postgres11 postgres12

dev-sqlite:
	echo 'sqlite' > current_connector

dev-postgres9:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres9
	echo 'postgres9' > current_connector

dev-postgres10:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres10
	echo 'postgres10' > current_connector

dev-postgres11:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres11
	echo 'postgres11' > current_connector

dev-postgres12:
	docker-compose -f docker-compose.yml up -d --remove-orphans postgres12
	echo 'postgres12' > current_connector

dev-pgbouncer:
	docker-compose -f docker-compose.yml up -d --remove-orphans pgbouncer postgres11
	echo 'pgbouncer' > current_connector

dev-mysql:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-5-7
	echo 'mysql' > current_connector

dev-mysql8:
	docker-compose -f docker-compose.yml up -d --remove-orphans mysql-8-0
	echo 'mysql8' > current_connector

dev-mariadb:
	docker-compose -f docker-compose.yml up -d --remove-orphans mariadb-10-0
	echo 'mariadb' > current_connector

dev-down:
	docker-compose -f docker-compose.yml down -v --remove-orphans

use-local-migration-engine:
	cargo build --release
	cp target/release/migration-engine $(PRISMA2_BINARY_PATH)/

use-local-query-engine:
	cargo build --release
	cp target/release/prisma $(PRISMA2_BINARY_PATH)/runtime/
	cp target/release/prisma $(PRISMA2_BINARY_PATH)/query-engine-darwin
