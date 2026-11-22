## Scripts to rule them all. 

Inspired by https://github.com/github/scripts-to-rule-them-all

> If your scripts are normalized by name across all of your projects, your contributors only need to know the pattern, not a deep knowledge of the application. This means they can jump into a project and make contributions without first learning how to bootstrap the project or how to get its tests to run.

If instead you are a user of the [nix](https://nixos.org/manual/nix/stable/) package manager, you might find interesting packages and in the [nix](../nix/) directory.

### The scripts

#### script/bootstrap-darwin

[`script/bootstrap-darwin`](bootstrap-darwin) is used for fulfilling dependencies of the project and ensure it builds correctly.

The goal is to make sure all required dependencies are installed, and the workspace builds.

#### script/profile-shell

[`script/profile-shell`](profile-shell) allows to run Valgrind profiling over any example in the primsa-engines repository, using a linux docker image as a bridge.

For example, say we want to use [`massif`](https://valgrind.org/docs/manual/ms-manual.html) to valgrind heap allocations:

First, we can build the latest version of the repository in linux.

```
❯ script/profile-shell
==> Running command: docker build -t prisma-engines-profile:latest --build-arg RUST_VERSION=1.68.0 -f script/.profile-shell/Dockerfile .
[+] Building 1.0s (16/16) FINISHED
==> Running command: docker run -it --rm -v /Users/miguel/GitHub/prisma/prisma-engines:/prisma-engines -v /Users/miguel/.cargo:/.cargo -w /prisma-engines prisma-engines-profile:latest

# Now we build the prisma-engines, build artifacts will get cached
# in the /prisma-engines/target-alternatives/profiling directory

root@42b8f52761fb:/prisma-engines# build
==> Running command: cargo build --profile=profiling --examples --benches
    Finished profiling [optimized + debuginfo] target(s) in 25.89s
warning: the following packages contain code that will be rejected by a future version of Rust: connection-string v0.1.13
note: to see what the problems were, use the option 

# Once built, we can profile one of the examples
# in the /prisma-engines/target-alternatives/profiling directory


root@42b8f52761fb:/prisma-engines# valgrind --tool=massif target-alternatives/profiling/examples/schema_builder_build_odoo
==12== Massif, a heap profiler
==12== Copyright (C) 2003-2017, and GNU GPL'd, by Nicholas Nethercote
==12== Using Valgrind-3.14.0 and LibVEX; rerun with -h for copyright info
==12== Command: target-alternatives/profiling/examples/schema_builder_build_odoo
==12==
Elapsed: 3.82s
==12==

# And visualize the profiliong results using any tool provided by the 
# valgrant package

root@42b8f52761fb:/prisma-engines# ms_print /prisma-engines/massif.out.11 |tail -n10
->01.42% (2,305,284B) 0x151446: <alloc::string::String as core::clone::Clone>::clone (alloc.rs:95)
  ->01.42% (2,305,284B) in 42 places, all below massif's threshold (1.00%)

--------------------------------------------------------------------------------
  n        time(i)         total(B)   useful-heap(B) extra-heap(B)    stacks(B)
--------------------------------------------------------------------------------
 51  2,400,085,093      140,932,776      124,442,047    16,490,729            0
 52  2,423,804,986       94,167,552       83,189,217    10,978,335            0
 53  2,447,524,833       47,307,048       41,851,522     5,455,526            0
 54  2,471,244,705        4,387,240        4,122,973       264,267            0
 ```