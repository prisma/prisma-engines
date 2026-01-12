# Query Plan Caching: Schema-Aware Parameterization

## Overview

This document describes the implementation of schema-aware parameterization for Prisma's query plan caching feature. The goal is to explicitly mark which input fields in the query schema can accept placeholder values (parameters) vs. which fields are structural parts of the query.

## Problem Statement

The current query parameterization is approximate and fails for certain cases. We need proper schema-aware parameterization where:

1. The schema explicitly describes where placeholders are allowed
2. The query parser validates placeholder usage against the schema
3. The DMMF exposes parameterization information for client code generation

## Key Concepts

### Parameterizable vs Structural Fields

**Parameterizable fields** accept user data that can vary between query executions without changing the query plan:
- Filter values (`where: { id: { equals: $id } }`) - `equals`, `not`, `in`, `notIn`, `lt`, `lte`, `gt`, `gte`
- String filter values - `contains`, `startsWith`, `endsWith`, `search`
- List filter values - `has`, `hasEvery`, `hasSome`
- JSON filter values - `arrayContains`, `arrayStartsWith`, `arrayEndsWith`, `stringContains`, `stringStartsWith`, `stringEndsWith`
- Data values in create/update operations - scalar field values
- Numeric update operations - `set`, `increment`, `decrement`, `multiply`, `divide`
- List update operations - `set`, `push`

> **Important:** Only scalar fields can be parameterizable, never object types. Objects like `where`, `cursor`, `having` are wrappers - the scalar fields *inside* them are automatically parameterizable via type reuse.

## Key Architectural Insight

**Parameterization is a leaf-node property.** Only scalar fields (the leaves of the input tree) can be marked as parameterizable. Object types are just structural wrappers and are never parameterizable themselves.

This has important implications:

1. **We only mark leaf scalar fields** - Fields like `equals`, `contains`, `set`, `increment` in filter types and data mappers
2. **Type reuse handles propagation** - When `cursor` reuses `WhereUniqueInput`, or `having` reuses filter types, the scalar fields inside are already marked as parameterizable
3. **No inheritance needed** - Each `InputField` has its own `is_parameterizable` flag set at definition time; there's no runtime propagation from parent to child
4. **Parser validation is straightforward** - When parsing a value, we check the `is_parameterizable` flag of the `InputField` being parsed; placeholders are only valid when parsing a scalar field marked as parameterizable

Example traversal for `where: { id: { equals: $param } }`:
- `where` → object wrapper, NOT parameterizable (but we don't check - it's not a scalar)
- `id` → object wrapper (filter type), NOT parameterizable (but we don't check - it's not a scalar)  
- `equals` → **scalar field, IS parameterizable** ✓ → placeholder allowed here

**Structural fields** affect the shape/structure of the query plan and cannot be parameterized:
- `take` / `skip` (converted to `i64`, used directly as SQL LIMIT/OFFSET integers)
- `orderBy` (contains field references and sort direction enums, not user values)
- `distinct` (contains field references, not user values)
- `by` in groupBy (contains field references, not user values)
- `select` / `include` (already handled separately, not in input types)

**Object wrappers** (the argument itself is not parameterizable, but scalar fields inside are automatically parameterizable via type reuse):
- `cursor` - reuses `WhereUniqueInput`, scalar fields inside flow to SQL via `db_values()`
- `having` - reuses filter types from `field_filter_types.rs`
- `where` - reuses filter types from `field_filter_types.rs`

**Fields converted to Rust primitives** (would fail if given a placeholder):
- `mode` - converted to `QueryMode` enum for case sensitivity
- `path` (JSON) - converted to `JsonFilterPath` struct
- `isEmpty` - converted to `bool`
- `isSet` - converted to `bool`
- `unset` - converted to `bool`

**Relation filter wrappers** (take nested objects, not scalar values):
- `some`, `every`, `none` - relation list filters
- `is`, `isNot` - to-one relation filters

## Chosen Approach: Field-Level Flag (Option B)

We chose to add an `is_parameterizable` boolean flag to `InputField` rather than modifying `InputType` because:

1. Parameterizability is about **where** in the query tree a value appears, not the type itself
2. The same type (e.g., `Int`) can be parameterizable in one context (`equals: 5`) but not in another (`take: 10`)
3. Simpler implementation with less invasive changes
4. JSON fields work naturally - the entire JSON value is treated as a single parameter

### Default Behavior

Fields are **not parameterizable by default**. Parameterizable fields must be explicitly marked with `.parameterizable()` in the schema builder. This is safer as it requires explicit opt-in for parameterization.

## Implementation Phases

1. **[Phase 1: Schema Infrastructure](./01-schema-infrastructure.md)** - Add `is_parameterizable` to `InputField`
2. **[Phase 2: DMMF Output](./02-dmmf-output.md)** - Expose flag in DMMF for client generator
3. **[Phase 3: Schema Builder](./03-schema-builder.md)** - Mark appropriate fields as parameterizable (excludes list-accepting fields)
4. **[Phase 3a: Relevance Search Prep](./03a-relevance-search-prep.md)** - Enable `orderBy._relevance.search` parameterization (optional follow-up)
5. **[Phase 3b: `in`/`notIn` Parameterization](./03b-in-notin-parameterization.md)** - Enable inclusion filter parameterization
6. **[Phase 3c: `hasSome`/`hasEvery` Parameterization](./03c-hassome-hasevery-parameterization.md)** - Enable scalar list filter parameterization
7. **[Phase 4: Query Parser Validation](./04-parser-validation.md)** - Validate placeholder usage

### Testing Strategy

Each phase includes its own tests that must be added alongside the implementation:
- **Phase 1**: Unit tests for `InputField` methods (`is_parameterizable()`, `parameterizable()`, `parameterizable_if()`)
- **Phase 2**: DMMF snapshot tests verifying `isParameterizable` appears in JSON output
- **Phase 3**: DMMF snapshot updates verifying correct fields are marked as parameterizable
- **Phase 4**: Parser validation tests + integration tests in connector-test-kit-rs

All existing tests must pass after completing each phase. End-to-end testing with Prisma Client will be possible once client-side parameterization logic is implemented (separate work).

### Note on List Parameterization

List-accepting fields (`in`, `notIn`, `hasSome`, `hasEvery`) require special handling because **lists must be parameterized as a whole value**, not element-by-element. This is because:

- For `in`/`notIn`: The SQL `IN` clause requires `ParameterizedRow` with dynamic expansion (`col IN [$1]` where `$1` is expanded to `$1, $2, ...` based on array length at execution time)
- For `hasSome`/`hasEvery`: PostgreSQL array operators (`@>`, `&&`) can accept array parameters directly as a single value

These are handled in Phases 3b and 3c after the core parameterization infrastructure is in place.

> **Note:** `has` takes a **single scalar value** (not a list), so it's parameterizable in the main Phase 3 just like `equals` or `contains`.

## File Map

Key files involved in this implementation:

```
query-compiler/
├── schema/src/
│   ├── input_types.rs          # InputField struct (Phase 1)
│   ├── build/
│   │   ├── utils.rs            # input_field() helper (Phase 1)
│   │   └── input_types/
│   │       └── fields/
│   │           ├── arguments.rs           # Pagination args (Phase 3)
│   │           ├── field_filter_types.rs  # Filter types (Phase 3)
│   │           ├── input_fields.rs        # Nested input fields (Phase 3)
│   │           └── data_input_mapper/     # Create/update data (Phase 3)
│   └── query_schema.rs         # ScalarType enum (reference)
├── dmmf/src/
│   ├── serialization_ast/
│   │   └── schema_ast.rs       # DmmfInputField (Phase 2)
│   └── ast_builders/
│       └── schema_ast_builder/
│           └── field_renderer.rs  # render_input_field (Phase 2)
└── core/src/
    └── query_document/
        └── parser.rs           # Placeholder validation (Phase 4)

libs/
└── user-facing-errors/src/
    └── query_engine/
        └── validation.rs       # New error type (Phase 4)
```

## Success Criteria

1. `InputField` has `is_parameterizable` flag with getter
2. DMMF output includes `isParameterizable` for all input fields
3. Filter/data fields marked as parameterizable
4. Pagination/structural fields remain non-parameterizable
5. Query parser rejects placeholders in non-parameterizable fields with clear error
6. All existing tests pass
7. New tests cover parameterization behavior

## Related Work

- **Prisma Client side**: Will consume DMMF's `isParameterizable` to build efficient parameterization data structures
- **Query Compiler**: Already handles `PrismaValue::Placeholder` in query plans
- **Existing `ScalarType::Param`**: Used for output types, not related to this work