# Prisma Rust

Development Stack: Rust & Cargo.

## Building Binaries in Debug Mode

This section contains instructions for building the binaries that are powering the [Prisma Framework](https://github.com/prisma/prisma2) with a [**development**](https://doc.rust-lang.org/book/ch14-01-release-profiles.html) release profile (i.e. in _debug mode_).

### 1. Clone the repository

First, you need to clone this repository and navigate into its root folder:

```
git clone git@github.com:prisma/prisma-engine.git
cd prisma-engine
```

### 2. Switch to the beta version of Rust

You can switch to Rust's beta version using the following command:

```
rustup default beta
```

Afterwards you can verify that the switch worked by running `rustc --version`. If your version includes `beta`, the switch was successful.

### 3. Build binaries in development mode

The development release profile is the default when you're building your code with [Cargo](https://doc.rust-lang.org/cargo/)'s `build` command. Therefore, you can build your project in debug mode as follows:

```
cargo build
```

### 4. Access the built binaries

You can find the compiled binaries inside the newly created `./target/debug` directory:

| Prisma Framework Component | Path to Binary |
| --- | --- |
| HTTP server + Query Engine | `./target/prisma/prisma` |
| Migration Engine | `./target/migration-engine/migration-engine` |
| Introspection Engine | `./target/introspection-engine/introspection-engine` |
| Prisma Format  |  `./target/prisma-fmt/prisma-fmt` 

## Coding Guidelines

* Prevent compiler warnings
* Use Rust formatting (`cargo fmt`)

## Testing

* To compile all modules use the provided `build.sh` script
* To test all modules use the provided `test.sh` script