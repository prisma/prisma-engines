# List of flaky tests

## Query Engine

- `new::regressions::prisma_15581::prisma_15581::create_one_model_with_low_precision_datetime_in_id`

Database: PostgreSQL 11

### Error

```
[2022-10-25T08:31:23Z] thread 'new::regressions::prisma_15581::prisma_15581::create_one_model_with_low_precision_datetime_in_id' panicked at 'assertion failed: result.contains(\"\\\"error\\\": \\\"Query createOnetest is required to return data, but found no record(s).\\\"\")', query-engine/connector-test-kit-rs/query-engine-tests/tests/new/regressions/prisma_15581.rs:73:9
[2022-10-25T08:31:23Z] stack backtrace:
[2022-10-25T08:31:23Z]    0: rust_begin_unwind
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/std/src/panicking.rs:584:5
[2022-10-25T08:31:23Z]    1: core::panicking::panic_fmt
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/panicking.rs:142:14
[2022-10-25T08:31:23Z]    2: core::panicking::panic
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/panicking.rs:48:5
[2022-10-25T08:31:23Z]    3: query_engine_tests::new::regressions::prisma_15581::prisma_15581::run_create_one_model_with_low_precision_datetime_in_id::{{closure}}
[2022-10-25T08:31:23Z]              at ./tests/new/regressions/prisma_15581.rs:73:9
[2022-10-25T08:31:23Z]    4: <core::future::from_generator::GenFuture<T> as core::future::future::Future>::poll
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/future/mod.rs:91:19
[2022-10-25T08:31:23Z]    5: <core::pin::Pin<P> as core::future::future::Future>::poll
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/future/future.rs:124:9
[2022-10-25T08:31:23Z]    6: query_tests_setup::run_connector_test_impl::{{closure}}
[2022-10-25T08:31:23Z]              at /root/build/query-engine/connector-test-kit-rs/query-tests-setup/src/lib.rs:283:28
[2022-10-25T08:31:23Z]    7: <core::future::from_generator::GenFuture<T> as core::future::future::Future>::poll
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/future/mod.rs:91:19
[2022-10-25T08:31:23Z]    8: <tracing_futures::WithDispatch<T> as core::future::future::Future>::poll::{{closure}}
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tracing-futures-0.2.5/src/lib.rs:455:47
[2022-10-25T08:31:23Z]    9: tracing_core::dispatcher::with_default
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tracing-core-0.1.29/src/dispatcher.rs:223:5
[2022-10-25T08:31:23Z]   10: <tracing_futures::WithDispatch<T> as core::future::future::Future>::poll
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tracing-futures-0.2.5/src/lib.rs:455:9
[2022-10-25T08:31:23Z]   11: <core::pin::Pin<P> as core::future::future::Future>::poll
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/future/future.rs:124:9
[2022-10-25T08:31:23Z]   12: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:525:48
[2022-10-25T08:31:23Z]   13: tokio::coop::with_budget::{{closure}}
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/coop.rs:102:9
[2022-10-25T08:31:23Z]   14: std::thread::local::LocalKey<T>::try_with
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/std/src/thread/local.rs:445:16
[2022-10-25T08:31:23Z]   15: std::thread::local::LocalKey<T>::with
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/std/src/thread/local.rs:421:9
[2022-10-25T08:31:23Z]   16: tokio::coop::with_budget
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/coop.rs:95:5
[2022-10-25T08:31:23Z]   17: tokio::coop::budget
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/coop.rs:72:5
[2022-10-25T08:31:23Z]   18: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:525:25
[2022-10-25T08:31:23Z]   19: tokio::runtime::scheduler::current_thread::Context::enter
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:349:19
[2022-10-25T08:31:23Z]   20: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:524:36
[2022-10-25T08:31:23Z]   21: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:595:57
[2022-10-25T08:31:23Z]   22: tokio::macros::scoped_tls::ScopedKey<T>::set
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/macros/scoped_tls.rs:61:9
[2022-10-25T08:31:23Z]   23: tokio::runtime::scheduler::current_thread::CoreGuard::enter
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:595:27
[2022-10-25T08:31:23Z]   24: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:515:19
[2022-10-25T08:31:23Z]   25: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:161:24
[2022-10-25T08:31:23Z]   26: tokio::runtime::Runtime::block_on
[2022-10-25T08:31:23Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/mod.rs:490:46
[2022-10-25T08:31:23Z]   27: query_tests_setup::run_with_tokio
[2022-10-25T08:31:23Z]              at /root/build/query-engine/connector-test-kit-rs/query-tests-setup/src/lib.rs:56:5
[2022-10-25T08:31:23Z]   28: query_tests_setup::run_connector_test_impl
[2022-10-25T08:31:23Z]              at /root/build/query-engine/connector-test-kit-rs/query-tests-setup/src/lib.rs:268:5
[2022-10-25T08:31:23Z]   29: query_tests_setup::run_connector_test
[2022-10-25T08:31:23Z]              at /root/build/query-engine/connector-test-kit-rs/query-tests-setup/src/lib.rs:221:5
[2022-10-25T08:31:23Z]   30: query_engine_tests::new::regressions::prisma_15581::prisma_15581::create_one_model_with_low_precision_datetime_in_id
[2022-10-25T08:31:23Z]              at ./tests/new/regressions/prisma_15581.rs:64:5
[2022-10-25T08:31:23Z]   31: query_engine_tests::new::regressions::prisma_15581::prisma_15581::create_one_model_with_low_precision_datetime_in_id::{{closure}}
[2022-10-25T08:31:23Z]              at ./tests/new/regressions/prisma_15581.rs:64:5
[2022-10-25T08:31:23Z]   32: core::ops::function::FnOnce::call_once
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/ops/function.rs:248:5
[2022-10-25T08:31:23Z]   33: core::ops::function::FnOnce::call_once
[2022-10-25T08:31:23Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/ops/function.rs:248:5
[2022-10-25T08:31:23Z] note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

## Query Engine

- `new::interactive_tx::interactive_tx::tx_expiration_cycle`

Database: MySQL 5.6

### Error

```
[2022-10-25T08:29:42Z] KNOWN ERROR KnownError { message: "Transaction API error: Transaction already closed: Could not perform operation.", meta: Object {"error": String("Transaction already closed: Could not perform operation.")}, error_code: "P2028" }
[2022-10-25T08:29:42Z] thread 'new::interactive_tx::interactive_tx::tx_expiration_cycle' panicked at 'assertion failed: known_err.message.contains(\"A commit cannot be executed on a closed transaction.\")', query-engine/connector-test-kit-rs/query-engine-tests/tests/new/interactive_tx.rs:92:9
[2022-10-25T08:29:42Z] stack backtrace:
[2022-10-25T08:29:42Z]    0: rust_begin_unwind
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/std/src/panicking.rs:584:5
[2022-10-25T08:29:42Z]    1: core::panicking::panic_fmt
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/panicking.rs:142:14
[2022-10-25T08:29:42Z]    2: core::panicking::panic
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/panicking.rs:48:5
[2022-10-25T08:29:42Z]    3: query_engine_tests::new::interactive_tx::interactive_tx::run_tx_expiration_cycle::{{closure}}
[2022-10-25T08:29:42Z]              at ./tests/new/interactive_tx.rs:92:9
[2022-10-25T08:29:42Z]    4: <core::future::from_generator::GenFuture<T> as core::future::future::Future>::poll
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/future/mod.rs:91:19
[2022-10-25T08:29:42Z]    5: <core::pin::Pin<P> as core::future::future::Future>::poll
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/future/future.rs:124:9
[2022-10-25T08:29:42Z]    6: query_tests_setup::run_connector_test_impl::{{closure}}
[2022-10-25T08:29:42Z]              at /root/build/query-engine/connector-test-kit-rs/query-tests-setup/src/lib.rs:283:28
[2022-10-25T08:29:42Z]    7: <core::future::from_generator::GenFuture<T> as core::future::future::Future>::poll
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/future/mod.rs:91:19
[2022-10-25T08:29:42Z]    8: <tracing_futures::WithDispatch<T> as core::future::future::Future>::poll::{{closure}}
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tracing-futures-0.2.5/src/lib.rs:455:47
[2022-10-25T08:29:42Z]    9: tracing_core::dispatcher::with_default
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tracing-core-0.1.29/src/dispatcher.rs:223:5
[2022-10-25T08:29:42Z]   10: <tracing_futures::WithDispatch<T> as core::future::future::Future>::poll
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tracing-futures-0.2.5/src/lib.rs:455:9
[2022-10-25T08:29:42Z]   11: <core::pin::Pin<P> as core::future::future::Future>::poll
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/future/future.rs:124:9
[2022-10-25T08:29:42Z]   12: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:525:48
[2022-10-25T08:29:42Z]   13: tokio::coop::with_budget::{{closure}}
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/coop.rs:102:9
[2022-10-25T08:29:42Z]   14: std::thread::local::LocalKey<T>::try_with
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/std/src/thread/local.rs:445:16
[2022-10-25T08:29:42Z]   15: std::thread::local::LocalKey<T>::with
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/std/src/thread/local.rs:421:9
[2022-10-25T08:29:42Z]   16: tokio::coop::with_budget
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/coop.rs:95:5
[2022-10-25T08:29:42Z]   17: tokio::coop::budget
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/coop.rs:72:5
[2022-10-25T08:29:42Z]   18: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:525:25
[2022-10-25T08:29:42Z]   19: tokio::runtime::scheduler::current_thread::Context::enter
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:349:19
[2022-10-25T08:29:42Z]   20: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:524:36
[2022-10-25T08:29:42Z]   21: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:595:57
[2022-10-25T08:29:42Z]   22: tokio::macros::scoped_tls::ScopedKey<T>::set
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/macros/scoped_tls.rs:61:9
[2022-10-25T08:29:42Z]   23: tokio::runtime::scheduler::current_thread::CoreGuard::enter
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:595:27
[2022-10-25T08:29:42Z]   24: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:515:19
[2022-10-25T08:29:42Z]   25: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/scheduler/current_thread.rs:161:24
[2022-10-25T08:29:42Z]   26: tokio::runtime::Runtime::block_on
[2022-10-25T08:29:42Z]              at /root/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.21.0/src/runtime/mod.rs:490:46
[2022-10-25T08:29:42Z]   27: query_tests_setup::run_with_tokio
[2022-10-25T08:29:42Z]              at /root/build/query-engine/connector-test-kit-rs/query-tests-setup/src/lib.rs:56:5
[2022-10-25T08:29:42Z]   28: query_tests_setup::run_connector_test_impl
[2022-10-25T08:29:42Z]              at /root/build/query-engine/connector-test-kit-rs/query-tests-setup/src/lib.rs:268:5
[2022-10-25T08:29:42Z]   29: query_tests_setup::run_connector_test
[2022-10-25T08:29:42Z]              at /root/build/query-engine/connector-test-kit-rs/query-tests-setup/src/lib.rs:221:5
[2022-10-25T08:29:42Z]   30: query_engine_tests::new::interactive_tx::interactive_tx::tx_expiration_cycle
[2022-10-25T08:29:42Z]              at ./tests/new/interactive_tx.rs:64:5
[2022-10-25T08:29:42Z]   31: query_engine_tests::new::interactive_tx::interactive_tx::tx_expiration_cycle::{{closure}}
[2022-10-25T08:29:42Z]              at ./tests/new/interactive_tx.rs:64:5
[2022-10-25T08:29:42Z]   32: core::ops::function::FnOnce::call_once
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/ops/function.rs:248:5
[2022-10-25T08:29:42Z]   33: core::ops::function::FnOnce::call_once
[2022-10-25T08:29:42Z]              at /rustc/a55dd71d5fb0ec5a6a3a9e8c27b2127ba491ce52/library/core/src/ops/function.rs:248:5
[2022-10-25T08:29:42Z] note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```
