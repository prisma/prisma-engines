# Prisma Engines

This repository contains a collection of engines that power the core stack for the [Prisma Framework](https://github.com/prisma/prisma2), most prominently [Photon](https://github.com/prisma/photonjs/) and [Lift](https://github.com/prisma/lift/).

The engines and their respective binary crates are:
- Query engine: `prisma`
- Migration engine: `migration-engine`
- Introspection engine: `introspection-engine`
- Prisma Format: `prisma-fmt`

## Building Prisma Engines

**Prerequisites:**
- Installed the stable Rust toolchain, at least version 1.39.0. You can get the toolchain at [rustup](https://rustup.rs/) or the package manager of your choice.
- Linux only: OpenSSL is required to be installed.

**How to build:**

To build all engines, simply execute `cargo build` on the repository root. This builds non-production debug binaries.
If you want to build the optimized binaries in release mode, the command is `cargo build --release`.

Depending on how you invoked `cargo` in the previous step, you can find the compiled binaries inside the repository root in the `target/debug` (without `--release`) or `target/release` directories (with `--release`):

| Prisma Framework Component | Path to Binary                                            |
| -------------------------- | --------------------------------------------------------- |
| Query Engine               | `./target/[debug|release]/prisma`                         |
| Migration Engine           | `./target/[debug|release]/migration-engine`               |
| Introspection Engine       | `./target/[debug|release]/introspection-engine`           |
| Prisma Format              | `./target/[debug|release]/prisma-fmt`                     |

## Testing

There are two test suites for the engines: Unit tests and integration tests.

Unit tests are implemented in the Rust code, and can be invoked with `cargo test -- --test-threads 1`.

## WIP Coding Guidelines
- Prevent compiler warnings
- Use Rust formatting (`cargo fmt`)
