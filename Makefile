default:
	cargo build

# Build the crates with deny-warnings on to emulate CI
pedantic:
	RUSTFLAGS="-D warnings" cargo build

release:
	cargo build --release

dev-all:
	docker-compose -f .buildkite/engine-build-cli/docker-test-setups/docker-compose.test.all.yml up -d --remove-orphans

dev-sqlite:
	make dev-all
	echo 'sqlite' > current_connector

dev-postgres:
	make dev-all
	echo 'postgres' > current_connector

dev-mysql:
	make dev-all
	echo 'mysql' > current_connector

dev-mysql8:
	make dev-all
	echo 'mysql8' > current_connector


use-local-migration-engine:
	cargo build --release
	cp target/release/migration-engine $(PRISMA2_BINARY_PATH)/

use-local-query-engine:
	cargo build --release
	cp target/release/prisma $(PRISMA2_BINARY_PATH)/runtime/
	cp target/release/prisma $(PRISMA2_BINARY_PATH)/query-engine-darwin
