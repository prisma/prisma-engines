## Scripts to rule them all. 

Inspired by https://github.com/github/scripts-to-rule-them-all

> If your scripts are normalized by name across all of your projects, your contributors only need to know the pattern, not a deep knowledge of the application. This means they can jump into a project and make contributions without first learning how to bootstrap the project or how to get its tests to run.

This scripts can also be run with make. Following the convention: `script/$TARGET` -> `make $TARGET`. For instance, make bootstrap-darwin will run `script/bootstrap-darwin`

If instead you are a user of the [nix](https://nixos.org/manual/nix/stable/) package manager, you might find interesting packages and in the [nix](../nix/) directory.

### The scripts

#### script/bootstrap-darwin

[`script/bootstrap-darwin`](bootstrap-darwin) is used for fulfilling dependencies of the project and ensure it builds correctly.

The goal is to make sure all required dependencies are installed, and the workspace builds.

#### script/profile

[`script/profile`](profile) allows to run Valgrind profiling over any example in the primsa-engines
repository, using a linux docker image as a bride.

Example. Let´s say we want to use [`massif`](https://valgrind.org/docs/manual/ms-manual.html) to profile heap allocations, same will apply to other tools:

```sh
> ~/GitHub/prisma/prisma-engines
❯ script/profile massif schema_builder_build_odoo
==> Profiling docker image (prisma-engines-profile:1.68.0.31f76) exists, using it
==> Running command: docker run -it -v /Users/miguel/GitHub/prisma/prisma-engines:/rustrepo -v /Users/miguel/.cargo:/.cargo -w /rustrepo prisma-engines-profile:1.68.0.31f76 bash -C build-and-run massif schema_builder_build_odoo
==> Running: build && run-valgrind massif schema_builder_build_odoo
==> Building examples to profile...
    Finished profiling [optimized + debuginfo] target(s) in 15.08s
warning: the following packages contain code that will be rejected by a future version of Rust: connection-string v0.1.13
note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 33`
==11== Massif, a heap profiler
==11== Copyright (C) 2003-2017, and GNU GPL'd, by Nicholas Nethercote
==11== Using Valgrind-3.14.0 and LibVEX; rerun with -h for copyright info
==11== Command: /rustrepo/target-alternatives/profiling/examples/schema_builder_build_odoo
==11==
Elapsed: 5.78s
==11==
```

The massif output file was generated in the own repository folder, which was mounted in the docker image

```sh
❯ git status
Untracked files:
	massif.out.11
```

Then use [`script/profile-shell`](profile-shell) to view the results using `ms-print` as the [docs](https://valgrind.org/docs/manual/ms-manual.html) suggest:

```

```sh
❯ script/profile-shell ms_print /rustrepo/massif.out.11 |tail -n10
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

#### script/profile-shell

[`script/profile-shell`](profile-shell). Use it to run arbitrary commands on the docker profiling image. See the example provided in [`script/profile`](profile)