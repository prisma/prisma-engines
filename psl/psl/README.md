# PSL stands for Prisma Schema Language

## Overview

This crate is the public API for working with Prisma schemas: parsing and
validating their string representation, analyzing them, providing convenient
APIs, rendering, reformatting, etc. It is the main implementation of the Prisma
Schema Language, and it is used by all Prisma engines in this repository.

The query engine further processes Prisma schemas to produce the client API,
and the DMMF JSON format.

## Usage

Please see [`lib.rs`](src/lib.rs) and the [rustdoc documentation](https://prisma.github.io/prisma-engines/doc/psl/) for the public API.

Main use-case, parsing a string to datamodel:

```ignore
let file = std::fs::read_to_string(&args[1]).unwrap();
let validated_schema = datamodel::parse_schema(&file)?;
```
