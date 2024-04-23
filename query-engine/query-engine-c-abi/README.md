# UNSTABLE/EXPERIMENTAL Query Engine C (compatible) ABI

This version of the query engine exposes the Rust engine via C callable functions. There are subtle differences to this implementation compared to the node and wasm versions. Although it is usable by any language that can operate with the C ABI, it is oriented to having prisma running on react-native so the build scripts are oriented to that goal.

## Setup

You need to have Xcode, Java, Android's NDK (you can/should install it via Android Studio), Cmake installed on your machine to compile the engine. The make file contains the main entry points for building the different architectures and platforms. You also need to install the target Rust architectures. You can find the exact [process described here](https://ospfranco.com/post/2023/08/11/react-native,-rust-step-by-step-integration-guide/).

- `make ios` → Builds the iOS libraries in release mode
- `make sim` → Builds the simulator arch only in debug, much faster, meant for rapid development
- `make android` → Builds all the android archs
- `make all` → Builds all the archs

Once the libraries have been built there are a couple of extra scripts (`copy-ios.sh` and `copy-android.sh`) that move the results of the compilation into a sibling of the parent folder (`react-native-prisma`), which is where they will be packaged and published to npm.

The result of the compilation are static libraries (.a) as well a generated C header file.

A C header file (`include/query_engine.h`) is automatically generated on the compilation process via `cbindgen`. There is no need to manually modify this file, it will be automatically generated and packaged each time you compile the library. You need to mark the functions inside `engine.rs` as `extern "C"` so that the generator picks them up.

### iOS

iOS requires the use of `.xcframework` to package similar architectures (proper iOS and iOS 64 bit simulator thanks to m1 machines) without conflicts.

## Base Path

This query engine takes one additional parameter in the create function (the entry point of all operations), which is the `base_path` string param. This param is meant to allow the query engine to change it's working directory to the passed path. This is required on iOS (and on the latest versions of Android) because the file system is sandboxed. The react-native client library that consumes this version of the engine passes the Library directory on iOS and the Databases folder on Android, both of this folders are within the sandbox and can be freely read and written. The implemented solution literally just changes the working directory of the Rust code in order to allow the query engine to operate as if it was working on a non-sandboxed platform and allowed to the query engine to run without changing implementation details and even hackier workarounds. It might have unintented consequences on the behavior of the engine though, so if you have any issues please report them.

## Migrations

This query engine version also contains parts of the schema engine. Previous versions of prisma were meant to be run on the server by the developer to test migrations or execute them for a single server database. Now that we are targeting front-end platforms, it is required to be able to perform migrations ON-DEVICE and on RUNTIME.

In order to enable this there are some new functions exposed through the query engine api that call schema engine.

- `prisma_apply_pending_migrations` → Given a path, it will scan all the folders in alphabetical order all look inside for a `migration.sql` and execute that. It's equivalent (it literally calls the same internal function) as `prisma migrate dev`

- `prisma_push_schema` → Will try to apply the passed schema into the database in an unsafe manner. Some data might be lost. It's equivalent to `prisma db push`

## Usage

Like any C-API, returning multiple chunks of data is done via passing pointers (e.g. SQLite). Especially the query engine instanciation, will return a obfuscated pointer allocated on the heap. You need to pass this pointer to each subsequent call to the interfaces to use the query engine functionality.

Each operation should return an integer status code that indicates PRISMA_OK (0) if the opereation finished correctly or different error codes for each possible error.

C calls are not compatible with tokio/async, so the C functions need to use `block_on` in order to keep synchronicity. If async functionality is wanted the calling language/environment should spin up their own threads and call the functions in there.

While `block_on` might not be the most efficient way to achieve things, it keeps changes to the core query_engine functionality at a minimum.

## OpenSSL Snafu

The query engine (to be exact, different database connectors) depends on OpenSSL, however, the Rust crate tries to compile the latest version which [currently has a problem with Android armv7 architectures](https://github.com/openssl/openssl/pull/22181). In order to get around this, we have to download OpenSSL, patch it, compile and link it manually. The download, patching and compiling is scripted via the `build-openssl.sh` script. You need to have the Android NDK installed and the `ANDROID_NDK_ROOT` variable set in your environment before running this script. You can find more info on the script itself. The libraries will be outputed in the `libs` folder with the specific structure the Rust compilation needs to finish linking OpenSSL in the main query engine compilation. The crate `openssl` then uses the compiled version by detecting the `OPENSSL_DIR` flag which is set in the `build-android-target.sh` script.

Once the issues upstream are merged we can get rid of this custom compilation step.

## Tests

The tests for React Native are dependant on JSI, meaning they cannot be run outside a device/simulator. The example app contains an HTTP server and the test setup has been reworked to send the requests via HTTP. The usual steps to running the tests are needed but you also need to be running the app and replace the IP address that appears on the screen in the `executor/rn.ts` file.
