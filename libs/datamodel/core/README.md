# Prisma Schema Language implementation

## Overview

This crate is the public API for working with Prisma schemas: parsing and
validating their string representation, analyzing them, providing convenient
APIs, rendering, reformatting, etc. It is the main implementation of the Prisma
Schema Language, and it is used by all Prisma engines in this repository.

The query engine further processes Prisma schemas to produce the client API,
and the DMMF JSON format.

Here is a (slightly dated) overview diagram:

![Architecture Overview](doc/images/overview.png?raw=true)

### Design goals

- Strict parsing: a duplicate attribute, unknown attribute, unknown argument or extra argument is an error.
- Expose a _convenient_ and _obvious_ / hard-to-misuse public API.
- Expose the source information (AST node spans, etc) in the parsed schema.
- Accumulate errors to present them at the end instead of returning early.

### Data Formats

**Sources** represents the different datasources declared in the schema.

**DML** is a datamodel data structure which is used by other Rust components of Prisma.

**AST** is the internal AST representation of a Prisma datamodel V2 file.

**Datamodel V2 String** is the string representation of a Prisma datamodel V2 file.

**DMMF** Internal JSON format for transferring datamodel and schema information
between different components of Prisma. The DMMF is in parts in the `dmmf`
crate in datamodel, but for the most part defined in the query engine.

### Steps

**Parse** parses a string to an AST and performs basic syntactic checks.

**Load Sources** Loads all datasource and generator declarations. This injects
source-specific attributes into the validation pipeline.

**Validate** performs several checks to ensure the datamodel is valid. This
includes, for example, checking invalid type references, or relations which are
impossible to model on a database.

**Lift** converts a validated schema to a DML struct. That step cannot fail, it
does not perform any validation.

**Lower** generates an AST from a DML struct. This step will attempt to
minimize the AST by removing all attributes and attribute arguments which are a
default anyway.

**Render** renders a given AST to its string representation.

## Error handling

The datamodel parser strives to provide good error diagnostics to schema
authors. As such, it has to be capable of dealing with partially broken input
and keep validating. The validation process can however not proceed to the
validating models in presence of a broken datasource, for example, because that
would lead to a cascade of misleading errors. Like other parsers, we introduce
*cutoff points* where validation will stop in the presence of errors.

These are:

- AST parsing stage. Syntax errors.
- Configuration validation
- Datamodel validation

## Usage

Please see [`lib.rs`](src/lib.rs) and the [rustdoc documentation](https://prisma.github.io/prisma-engines/doc/datamodel/) for all convenience methods.

Main use-case, parsing a string to datamodel:

```
let file = std::fs::read_to_string(&args[1]).unwrap();
let validated_schema = datamodel::parse_schema_parserdb(&file)?;
```
