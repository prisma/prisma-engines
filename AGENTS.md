# Agent Playbook — Prisma Engines

## 1. Big Picture
- This repo hosts the **Prisma Engines**: PSL (schema parser/validator), schema-engine (migrate, introspect), query components (legacy query engine, new query compiler), driver adapters, and utilities shared with Prisma Client.
- Prisma 7 roadmap status:
  - `directUrl` and `shadowDatabaseUrl` are **invalid** in PSL.
  - `url` remains temporarily (legacy query engine still consumes it); removal is a follow-up.
  - CLI/tests override connection info via schema-engine CLI (`--datasource`) or shared `TestApi::new_engine_with_connection_strings`.
  - Reference commit: `34b5a692b7bd79939a9a2c3ef97d816e749cda2f` (driver adapter override plumbing).
- Prisma is deprecating the **native Rust query engine** in favor of the **Query Compiler (QC)** architecture:
  - Query planning happens in Rust (`query-compiler` crate). Output: an expression tree (“query plan”).
  - Query interpretation/execution runs in Prisma Client TypeScript using driver adapters. The interpreter has no knowledge of connection strings or even whether it talks to a real DB.
  - A compatibility harness (`qc-test-runner.ts` in the main repo) emulates legacy query engine behavior for the test suite until QE removal is complete.
  - MongoDB support is not yet implemented for QC; Prisma 7 will ship without MongoDB, to be added later.

---

## 2. Repository Orientation
Key directories:
- `psl/` – Prisma Schema Language parser, validator, config tooling.
- `schema-engine/` – Migration/introspection engine plus test suites.
- `prisma-fmt/` – Language server & formatter entry point (tests rely on `expect!` snapshots).
- `libs/` – Shared libraries (metrics, value types, test setup).
- `schema-engine/sql-migration-tests` / `sql-introspection-tests` – Heavy integration suites (require DBs).
- `query-engine/` – Legacy query execution stack (Rust).
- `query-compiler/` – New query planner + associated WASM + playground.
- `libs/` – Shared libraries (metrics, value types, driver adapters, test setup).
- `driver-adapters/` – Rust-side adapter utilities for the new query interpreter.

Supporting infra:
- Tests use Rust `cargo test`. Some suites expect database URLs in env (see §5).
- `test-setup` crate provisions databases when env vars are defined (Docker-based in CI).
- `UPDATE_EXPECT=1 cargo test …` regenerates `expect!` snapshots (common when diagnostics shift).

---

## 3. Current Domain Knowledge
### Datasource URLs
- PSL rejects `directUrl`/`shadowDatabaseUrl` with targeted diagnostics (`DatamodelError::new_datasource_*_removed_error`).
- Parser still records `url` (and uses span for override fallbacks).
- `Datasource::override_urls()` now fakes spans because overrides bypass PSL parsing.
- Schema-engine tests must supply overrides via `TestApi::new_engine_with_connection_strings(connection_string, Some(shadow_connection))`. The wrapper returns an `EngineTestApi`.
- Old fixtures relying on `directUrl` inside PSL must be rewritten or deleted.
- Query compiler already assumes datasource URLs are supplied externally (from Prisma Client).

### Text Completions
- `prisma-fmt` completions now only offer `url` (no more direct/shadow suggestions).
- Completion scenarios removed for the deprecated properties. Expect JSON fixtures to change if docs/completions change again.

### Diagnostics / Tests
- Many tests assert on colored output via `expect!`. Always regenerate expectations when diagnostics wording changes.
- Integration tests around multi-schema migrations still need real DB URLs; without them they skip/fail early.
- Query compiler tests use insta snapshots (`query-compiler/tests`). Regenerate with `UPDATE_EXPECT=1 cargo test -p query-compiler`.
- Query engine connector tests rely on `cargo insta` snapshots too (see `connector-test-kit-rs` README).

---

## 4. Typical Workflows
### Linting / Formatting
- Rustfmt + cargo fmt (standard). JSON fixtures kept raw (no formatter).
- Full lint pass (formatting + clippy warnings as errors):
  ```bash
  make pedantic
  ```
  This runs `cargo fmt -- --check` and `cargo clippy --all-features --all-targets -Dwarnings`. Fix the compiler/clippy diagnostics first, then formatting.

### Running Tests
1. **Fast PSL/LSP suites**
   ```bash
   cargo test -p prisma-fmt -F psl/all
   ```
   Use `UPDATE_EXPECT=1` to refresh snapshots.

2. **Unit tests in PSL**
   ```bash
   cargo test -p psl -F all
   ```

3. **Unit tests for the whole workspace**
  ```bash
  make test-unit
  ```
  Use this one if you can't figure out the correct cargo features for a specific crate.
  Some library crates may be tricky to compile in isolation without feature unification.
  Unit tests are very fast so there's no problem running them for the whole workspace.
  Note that `cargo test` for the whole workspace won't work because of the Node-API
  symbol dependencies in the `query-engine-node-api` crate, use the makefile target.

3. **Schema engine SQL tests**
   Require DB env vars (see `.test_database_urls/` in repo root). Example:
   ```bash
   source .test_database_urls/postgres
   cargo test -p sql-migration-tests migration_with_shadow_database -- --nocapture
   ```

4. **Schema engine integration**
  Similar pattern; rely on generated DB URLs. Without env vars tests will refuse to run (by design).

5. **Query compiler snapshots**
   ```bash
   UPDATE_EXPECT=1 cargo test -p query-compiler
   ```
   Graphviz (`dot`) optional; set `RENDER_DOT_TO_PNG` for visuals (requires Graphviz installed).

6. **Query engine connector tests**
   ```bash
   make dev-postgres15   # or appropriate make target to spin up DB & config
   cargo test -p query-engine-tests -- --nocapture
   ```
   Requires `.test_config` or env vars; see `query-engine/connector-test-kit-rs/README.md`.

7. **Query engine Node API / C-ABI builds**
   - Node addon: `cargo build -p query-engine-node-api`.
   - C-ABI (used by React Native): `cargo build -p query-engine-c-abi`.

### Updating expect! snapshots
```bash
UPDATE_EXPECT=1 cargo test -p prisma-fmt [optional::test::path]
```
Ensure diffs make sense and rerun without `UPDATE_EXPECT` to confirm.

---

## 5. Environment Essentials
- **Databases**: env vars follow `TEST_DATABASE_URL`, `TEST_SHADOW_DATABASE_URL`, etc. Use the `.test_database_urls/` helper scripts or docker-compose setup from team docs.
- **Linear tickets**: two key Prisma 7 projects – *Breaking Changes* and *New Features*. Search via Linear MCP server if context needed.
- **Feature flags**: driver adapters live behind configuration (`prisma.config.ts` with `engine: 'classic' | 'js'`). Schema engine CLI accepts `--datasource` JSON payload – reuse the structure from commit `34b5a69…`.
- **Graphviz (`dot`)**: optional but useful for rendering query graphs (required if `RENDER_DOT_TO_PNG` set in QC tests/playground).
- **Node.js**: required when working with query-engine Node bindings or QC interpreter harness.
- **Docker**: used for local DBs via `docker-compose.yml`; make targets (`make dev-postgres15`, `make start-mongo6`, etc.) orchestrate containers + config files.

---

## 6. Common Gotchas
- Running prisma-fmt tests after updating diagnostics **without** refreshing expect files will cause failures. Always run with `UPDATE_EXPECT=1`.
- Some fixtures expect **CRLF** endings (`create_missing_block_composite_type_crlf`). Avoid rewriting line endings when not necessary. If Git warns, restore file from `HEAD`.
- Integration tests bail with “Missing TEST_DATABASE_URL”. Set env vars or skip running them locally.
- `TestApi` inside `sql-migration-tests` exposes `new_engine_with_connection_strings`; use it to pass overrides.
- When touching overrides, update both PSL and schema-engine sides; they share assumptions about spans and optional URLs.
- Query compiler shares substantial code with query engine (e.g., `query_core`, `query_structure`). Changes in shared crates affect both paths—be mindful of feature flags.
- Many query engine tests still assume native QE; the `qc-test-runner` harness (in main repo) ensures QC behaves like QE for now. Expect follow-up cleanup once QE removal completes.
- MongoDB currently runs only on legacy QE; QC MongoDB support is pending. Avoid regressing existing QE tests until QC parity is achieved.

---

## 7. Useful Commands & Snippets
- Show diff for specific file:
  `git diff path/to/file.rs`
- Re-run single Rust test:
  `cargo test -p prisma-fmt validate::tests::validate_direct_url_direct_empty -- --nocapture`
- Search for legacy attributes:
  `rg "directUrl"`, `rg "shadowDatabaseUrl"`
- Build schema-engine CLI:
  `cargo build -p schema-engine-cli`
- Build query compiler WASM:
  `make build-qc-wasm`
- Build legacy query engine binary:
  `cargo build -p query-engine`
- Query compiler playground (generate plan + graph):
  `cargo run -p query-compiler-playground`

Prefer using Makefile targets that take care of setting up the environment correctly or running prerequisite commands.

---

## 8. Open Themes / Future Tasks
- Removing `url` from PSL will be a follow-up; expect similar pattern (PSL error + override path).
- Query engine removal: remaining QE dependencies/tests need to migrate to QC or be deleted once QE is gone.
- Additional schema-engine tests may need migration to the new override helper.
- Documentation updates (internal + public) should mirror code changes; check when editing diagnostics to keep docs consistent.
- Query compiler MongoDB support: implement translation path + driver adapters, update tests once ready.
- Post-QE cleanup: strip QE-specific branches in shared crates (`query_core`, `query_structure`), simplify driver adapter plumbing.

---

## 9. External References
- Prisma Config (`prisma.config.ts`) implementation lives in the main Prisma repo (`@prisma/config` package).
- Linear roadmap items for Prisma 7 (Breaking Changes, New Features) hold context.
- Commit `34b5a69…` – canonical example for datasource override wiring.
- Query engine connector test guide: `query-engine/connector-test-kit-rs/README.md`.
- QC playground usage: `query-compiler/query-compiler-playground/`.
- QC harness in Prisma repo: `packages/cli/src/__tests__/queryCompiler/qc-test-runner.ts` (mirrors QE behavior).

---

**When modifying anything involving diagnostics or fixtures:** run relevant tests, refresh expectations, and ensure Git diffs are readable (no accidental CRLF/encoding swaps). Keep this file updated whenever we discover new traps.***
