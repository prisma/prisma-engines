# Phase 3: Schema Builder Updates

## Objective

Mark appropriate input fields as parameterizable in the schema builder. This is the largest phase as it requires identifying all locations where user data fields (parameterizable) vs structural fields (not parameterizable) are created.

## Guiding Principle

- **Parameterizable**: Fields that accept user data values that can vary between query executions without affecting the query plan structure
- **Not parameterizable**: Fields that affect query structure, ordering, pagination, or aggregation

## Key Architectural Insight

**Parameterization is a leaf-node property.** Only scalar fields (the leaves of the input tree) can be marked as parameterizable. Object types are structural wrappers and are never parameterizable themselves.

This means:

1. **We only mark leaf scalar fields** - Fields like `equals`, `contains`, `set`, `increment` that directly accept user values
2. **Type reuse handles propagation** - When `cursor` reuses `WhereUniqueInput`, or `having` reuses filter types, the scalar fields inside are already marked as parameterizable from their original definitions
3. **No inheritance needed** - Each `InputField` has its own `is_parameterizable` flag set at definition time; there's no runtime propagation from parent to child
4. **Object wrappers don't need the flag** - `where`, `cursor`, `having`, `data` are object wrappers; we don't mark them as parameterizable because placeholders can only appear in scalar positions

**Example:** For `where: { id: { equals: $param } }`:
- `where` → object wrapper, NOT parameterizable (doesn't matter - not a scalar)
- `id` → object wrapper (filter type), NOT parameterizable (doesn't matter - not a scalar)
- `equals` → **scalar field, IS parameterizable** ✓ → placeholder allowed here

---

## Important: List Fields Deferred

**List-accepting filter fields are NOT parameterizable in this phase.** They require special handling because:

1. **Lists must be parameterized as a whole value**, not element-by-element
2. For `in`/`notIn`: Uses `ParameterizedRow` with dynamic SQL expansion (`col IN [$1]` where `$1` is a variable-length array)
3. For `hasSome`/`hasEvery`: PostgreSQL array operators can accept array parameters directly

The following fields are explicitly **NOT parameterizable** in this phase:
- `in` / `notIn` - See [Phase 3b](./03b-in-notin-parameterization.md)
- `hasSome` / `hasEvery` - See [Phase 3c](./03c-hassome-hasevery-parameterization.md)

> **Note:** `has` takes a single scalar value (not a list) and IS parameterizable in this phase - same as `equals`, `contains`, etc.

---

## Files to Modify

### Primary Files (Structural fields - ensure NOT parameterizable)

1. `query-compiler/schema/src/build/input_types/fields/arguments.rs`
   - Pagination arguments: `take`, `skip` (not `cursor` values though)
   - `orderBy` arguments
   - `distinct` arguments
   - `by` argument (groupBy)

### Primary Files (Data fields - mark as parameterizable)

2. `query-compiler/schema/src/build/input_types/fields/field_filter_types.rs`
   - Comparison operations: `equals`, `not`, `lt`, `lte`, `gt`, `gte`
   - String filters: `contains`, `startsWith`, `endsWith`, `search`
   - Scalar list filter: `has` (single value check - NOT a list field)
   - JSON value filters: `arrayContains`, `arrayStartsWith`, `arrayEndsWith`, `stringContains`, `stringStartsWith`, `stringEndsWith`
   - **NOT in this phase**: `in`, `notIn`, `hasEvery`, `hasSome` (list fields - deferred)

3. `query-compiler/schema/src/build/input_types/fields/data_input_mapper/`
   - `create.rs` - Data fields in create operations
   - `update.rs` - Data fields in update operations

4. `query-compiler/schema/src/build/input_types/fields/input_fields.rs`
   - Nested relation data fields

---

## Task 3.1: Structural Arguments (NOT parameterizable)

These fields remain with the default `is_parameterizable: false`. Verify they are NOT marked as parameterizable.

**File:** `query-compiler/schema/src/build/input_types/fields/arguments.rs`

### Fields to verify:

```rust
// Pagination - NOT parameterizable (converted to i64, used as LIMIT/OFFSET integers)
pub(crate) fn take_argument<'a>(model: &Model) -> InputField<'a> {
    pagination_argument(args::TAKE, model)  // No .parameterizable()
}

pub(crate) fn skip_argument<'a>(model: &Model) -> InputField<'a> {
    pagination_argument(args::SKIP, model)  // No .parameterizable()
}

// Cursor - the cursor argument itself is an OBJECT wrapper, not a scalar, so it does NOT get .parameterizable()
// However, the scalar fields INSIDE cursor ARE parameterizable - this happens automatically because
// cursor reuses the WhereUniqueInput type, which contains scalar filter fields that we mark as parameterizable
// in field_filter_types.rs. The values flow as PrismaValue → SelectionResult → db_values() → SQL bound parameters.
input_field(args::CURSOR, vec![unique_input_type], None).optional()
// No .parameterizable() here - it's an object wrapper. The scalar fields inside are parameterizable via type reuse.

// OrderBy - around line ~143-152
input_field(
    args::ORDER_BY.to_owned(),
    vec![InputType::list(order_object_type.clone()), order_object_type],
    None,
)
.optional()
// No .parameterizable()

// Distinct - around line ~236-239
input_field(args::DISTINCT, input_types, None).optional()
// No .parameterizable()

// GroupBy "by" field - around line ~178-183
input_field(
    args::BY,
    vec![InputType::list(field_enum_type.clone()), field_enum_type],
    None,
)
// No .parameterizable()

// Having filter in groupBy - around line ~184
input_field(args::HAVING, vec![filter_object], None).optional()
// No .parameterizable() here - it's an object wrapper. The scalar filter fields inside (equals, lt, etc.)
// are automatically parameterizable because HAVING reuses the same filter types from field_filter_types.rs.
```

---

## Task 3.2: Filter Fields (mark as parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/field_filter_types.rs`

This file builds the filter types like `IntFilter`, `StringFilter`, etc. Need to find where filter operation fields are created.

### Search for filter field creation:

```bash
grep -n "input_field\|simple_input_field" query-compiler/schema/src/build/input_types/fields/field_filter_types.rs
```

### Fields to mark as parameterizable:

Scalar comparison/filter value fields:
- `equals` 
- `not`
- `lt`, `lte`, `gt`, `gte`
- `contains`, `startsWith`, `endsWith`
- `search` (full-text search)
- `has` (scalar list single-element check - takes ONE value, not a list)

### Fields explicitly NOT parameterizable in this phase:

List-accepting fields (require whole-list parameterization - see follow-up tasks):
- `in` / `notIn` - [Phase 3b](./03b-in-notin-parameterization.md)
- `hasEvery` / `hasSome` - [Phase 3c](./03c-hassome-hasevery-parameterization.md)

### Example changes:

```rust
// Before:
simple_input_field(filters::EQUALS, input_type.clone(), None).optional()

// After:
simple_input_field(filters::EQUALS, input_type.clone(), None)
    .optional()
    .parameterizable()
```

### Fields that should NOT be parameterizable in filters:

- `mode` (case sensitivity mode) - converted to Rust `QueryMode` enum in `parse_query_mode()` (see `extractors/filters/mod.rs:321-333`)
- `path` (JSON path) - converted to Rust `JsonFilterPath` in `parse_json_path()` (see `extractors/filters/scalar.rs:591-609`)
- `isEmpty` - converted to `bool` via `input.try_into()?` (see `extractors/filters/scalar.rs:175`)
- `isSet` - converted to `bool` via `input.try_into()?` (see `extractors/filters/scalar.rs:170-174`)
- `in` / `notIn` - deferred to [Phase 3b](./03b-in-notin-parameterization.md)
- `hasEvery` / `hasSome` - deferred to [Phase 3c](./03c-hassome-hasevery-parameterization.md)

---

## Task 3.3: Scalar List Filter Fields

**File:** `query-compiler/schema/src/build/input_types/fields/field_filter_types.rs`

Look for scalar list filter creation (e.g., `Int[]` fields):

### Parameterizable in this phase:
- `has` (single value check) → takes a single scalar value, same as `equals`

### NOT parameterizable in this phase:

List filter fields that accept arrays are deferred:
- `hasEvery` (list of values) → [Phase 3c](./03c-hassome-hasevery-parameterization.md)
- `hasSome` (list of values) → [Phase 3c](./03c-hassome-hasevery-parameterization.md)
- `equals` on list fields → may need similar treatment

### NOT parameterizable (converted to bool in Rust):
- `isEmpty` → `input.try_into()? converts to bool`, used as `field.is_empty_list(bool)`

---

## Task 3.4: Create Data Fields (mark as parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/data_input_mapper/create.rs`

### `CreateDataInputFieldMapper::map_scalar`

Find scalar field creation and mark as parameterizable:

```rust
// Before (around line ~25-34):
input_field(sf.name().to_owned(), vec![enum_type, typ], sf.default_value())
    .optional_if(!sf.is_required() || sf.default_value().is_some() || sf.is_updated_at())

// After:
input_field(sf.name().to_owned(), vec![enum_type, typ], sf.default_value())
    .optional_if(!sf.is_required() || sf.default_value().is_some() || sf.is_updated_at())
    .parameterizable()
```

### `CreateDataInputFieldMapper::map_scalar_list`

```rust
// Scalar list set operation - mark as parameterizable
// Note: This sets the entire list at once, so the whole value is parameterizable
simple_input_field(operations::SET, cloned_typ.clone(), None)
    .parameterizable()
```

### `CreateDataInputFieldMapper::map_composite`

```rust
// Composite fields with set operation - mark the scalar values inside as parameterizable
// This may require drilling into the composite object type creation
```

---

## Task 3.5: Update Data Fields (mark as parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/data_input_mapper/update.rs`

Similar to create, but also includes:
- `set` operation → `WriteOperation::scalar_set(value)` - parameterizable
- `increment`, `decrement`, `multiply`, `divide` operations → `WriteOperation::scalar_add/substract/multiply/divide(value)` - parameterizable
- `push` operation (for lists) → `WriteOperation::scalar_add(value)` - **TBD** - may need special handling like other list operations

**NOT parameterizable:**
- `unset` operation → converted to `bool` via `value.as_boolean().unwrap()` (see `write_args_parser.rs:82`)
  - For scalars: `WriteOperation::scalar_unset(*value.as_boolean().unwrap())`
  - For composites: `WriteOperation::composite_unset(*should_unset)` (see `write_args_parser.rs:201-205`)

---

## Task 3.6: Nested Input Fields

**File:** `query-compiler/schema/src/build/input_types/fields/input_fields.rs`

### `filter_input_field` function (line ~7-18)

This creates filter fields for model fields. Note: This creates the field wrapper, not the filter operations inside. The filter operations are created in `field_filter_types.rs`.

```rust
// This is an object wrapper field, does NOT need .parameterizable()
// The scalar filter operations inside (equals, lt, etc.) are marked in field_filter_types.rs
input_field(field.name().to_owned(), types, None)
    .optional()
    .nullable_if(nullable)
// No .parameterizable() here
```

### Nested relation operations

Review nested operation fields like `connect`, `disconnect`, `create`, `update`:
- The WHERE conditions in these should be parameterizable (via type reuse)
- But the operation type itself (connect vs disconnect) is structural

---

## Task 3.7: Where Argument

**File:** `query-compiler/schema/src/build/input_types/fields/arguments.rs`

The `where` argument itself is an object, but it contains filter fields that should be parameterizable. The filter fields are created in `field_filter_types.rs` so marking them there should propagate.

```rust
// where_argument function - the argument itself doesn't need .parameterizable()
// because the individual filter fields inside will be marked
pub(crate) fn where_argument<'a>(ctx: &'a QuerySchema, model: &Model) -> InputField<'a> {
    let where_object = filter_objects::where_object_type(ctx, model.into());
    input_field(args::WHERE.to_owned(), vec![InputType::object(where_object)], None).optional()
    // No .parameterizable() here - it's an object wrapper
}
```

---

## Implementation Strategy

### Recommended Order

1. Start with filter types (`field_filter_types.rs`) - highest impact
2. Then data mappers (`create.rs`, `update.rs`) 
3. Then input fields (`input_fields.rs`)
4. Finally verify structural fields remain unmarked (`arguments.rs`)

### Testing Approach

After each file:
```bash
cargo build -p schema
cargo test -p schema
```

After all changes:
```bash
UPDATE_EXPECT=1 cargo test -p dmmf  # Update DMMF snapshots
cargo test -p dmmf                   # Verify snapshots pass
```

### DMMF Snapshot Verification

The DMMF snapshots are the primary verification mechanism for Phase 3. After marking fields as parameterizable:

1. **Run snapshot update:**
   ```bash
   UPDATE_EXPECT=1 cargo test -p dmmf
   ```

2. **Review snapshot diffs** to verify:
   - Filter fields (`equals`, `not`, `lt`, `lte`, `gt`, `gte`, `contains`, `startsWith`, `endsWith`, `search`, `has`) show `isParameterizable: true`
   - JSON filter value fields (`arrayContains`, etc.) show `isParameterizable: true`
   - Create/update data scalar fields show `isParameterizable: true`
   - Numeric update operations (`increment`, `decrement`, `multiply`, `divide`) show `isParameterizable: true`
   - Structural fields (`take`, `skip`, `orderBy`, `distinct`, `by`) show `isParameterizable: false`
   - Boolean fields (`isEmpty`, `isSet`, `unset`, `mode`) show `isParameterizable: false`
   - List filter fields (`in`, `notIn`, `hasEvery`, `hasSome`) show `isParameterizable: false` (deferred)

3. **Commit updated snapshots** after review

### Example DMMF Diff

Before Phase 3:
```json
{
  "name": "equals",
  "isParameterizable": false
}
```

After Phase 3:
```json
{
  "name": "equals",
  "isParameterizable": true
}
```

---

## Verification Checklist

### Parameterizable (should have `.parameterizable()`):

- [ ] Filter `equals` fields
- [ ] Filter `not` fields  
- [ ] Filter `lt`, `lte`, `gt`, `gte` fields
- [ ] String filter `contains`, `startsWith`, `endsWith` fields
- [ ] Full-text `search` field
- [ ] Scalar list `has` field (single value check)
- [ ] JSON filter value fields (`arrayContains`, etc.)
- [ ] Create data scalar fields
- [ ] Create data scalar list `set` fields
- [ ] Update data scalar fields
- [ ] Update numeric operation fields (`increment`, `decrement`, etc.)
- [ ] Update list `set` fields
- [ ] Scalar fields inside `cursor` - automatically parameterizable via WhereUniqueInput type reuse

### NOT Parameterizable - Deferred to Follow-up Tasks:

**List-accepting filter fields (require whole-list parameterization):**
- [ ] `in` / `notIn` - [Phase 3b](./03b-in-notin-parameterization.md)
- [ ] `hasEvery` / `hasSome` - [Phase 3c](./03c-hassome-hasevery-parameterization.md)

### NOT Parameterizable (default, no marking needed):

**Structural query arguments (converted to Rust integers or field references):**
- [ ] `take` argument - converted to `i64`, used as SQL LIMIT
- [ ] `skip` argument - converted to `i64`, used as SQL OFFSET
- [ ] `orderBy` argument - contains field references and sort direction enums, not user values
- [ ] `distinct` argument - contains field references (enum values), not user values
- [ ] `by` argument (groupBy) - contains field references, not user values
- [ ] `having` argument (groupBy) - object wrapper, scalar fields inside auto-parameterizable via filter type reuse

**Filter fields converted to Rust primitives (NOT parameterizable):**
- [ ] `mode` field - converted to `QueryMode` enum (`extractors/filters/mod.rs:321-333`)
- [ ] `path` field (JSON) - converted to `JsonFilterPath` (`extractors/filters/scalar.rs:591-609`)
- [ ] `isEmpty` field - converted to `bool` (`extractors/filters/scalar.rs:175`, `composite.rs:27`)
- [ ] `isSet` field - converted to `bool` (`extractors/filters/scalar.rs:170-174`)

**Update operation fields converted to Rust primitives:**
- [ ] `unset` field - converted to `bool` (`write_args_parser.rs:82, 201-205`)

**Relation filter wrappers (take nested objects, not values):**
- [ ] `some`, `every`, `none` - relation list filters
- [ ] `is`, `isNot` - to-one relation filters

---

## Edge Cases to Consider

1. **Composite types**: Fields inside composite objects that accept user values - these go through `parse_composite_writes` and ultimately become `PrismaValue` in `WriteOperation`
2. **Relation filters**: `some`, `every`, `none`, `is`, `isNot` are structural wrappers that take nested filter objects. The VALUES inside those nested filters can be parameterizable, but the wrapper fields themselves are not.
3. **JSON filters**: The JSON comparison values (`equals`, `arrayContains`, etc.) are parameterizable, but `path` is converted to `JsonFilterPath` in Rust and is NOT parameterizable.
4. **Boolean filter fields**: `isEmpty`, `isSet`, `unset` are all converted to Rust `bool` and are NOT parameterizable.
5. **Cursor**: The cursor argument is an object wrapper and does NOT get `.parameterizable()`. However, the scalar fields *inside* cursor (the unique field values) ARE parameterizable - this happens automatically because cursor reuses the `WhereUniqueInput` type, which contains scalar filter fields marked as parameterizable. The values flow to SQL as bound parameters via `SelectionResult.db_values()`.
6. **HAVING**: Similar to cursor, the `having` argument is an object wrapper. The scalar filter fields inside (like `equals`, `lt`, etc.) are automatically parameterizable because HAVING reuses the same filter types from `field_filter_types.rs`.
7. **Take/Skip**: Unlike cursor, `take` and `skip` are converted to `i64` at extraction time and used directly as SQL LIMIT/OFFSET integers. They are NOT parameterizable.
8. **OrderBy relevance search**: The `search` string in `orderBy: { _relevance: { search: "..." } }` is converted to a Rust `String` but does flow to SQL for full-text search. This is a borderline case - currently NOT parameterizable due to the `.into_string().unwrap()` conversion. **This will be addressed in [Phase 3a](./03a-relevance-search-prep.md) as a follow-up task.**
9. **List filter fields**: `in`, `notIn`, `hasEvery`, `hasSome` require whole-list parameterization. **Deferred to follow-up tasks [3b](./03b-in-notin-parameterization.md), [3c](./03c-hassome-hasevery-parameterization.md).** Note: `has` takes a single value and IS parameterizable in this phase.
10. **Raw queries**: Out of scope (handled separately)

## Code References for Verification

When in doubt, check how a field is processed in the query graph builder:

- **Parameterizable**: Value flows through `as_condition_value()` or `try_into::<PrismaValue>()` and becomes part of `Filter` or `WriteOperation`
- **NOT parameterizable**: Value is converted via `try_into::<bool>()`, `try_into::<QueryMode>()`, or similar Rust type conversion

---

## Follow-up Tasks for List Parameterization

List fields require special handling because they must be parameterized as a whole value, not element-by-element. This affects SQL generation:

| Field | SQL Pattern | Parameterization Approach |
|-------|-------------|---------------------------|
| `in`/`notIn` | `col IN ($1, $2, ...)` | `ParameterizedRow` - dynamic expansion at execution |
| `hasSome` | `col && $1` | Single array parameter (PostgreSQL native) |
| `hasEvery` | `col @> $1` | Single array parameter (PostgreSQL native) |

> **Note:** `has` is NOT in this table because it takes a single scalar value, not a list. It's parameterizable in the main Phase 3.

See:
- [Phase 3b: `in`/`notIn` Parameterization](./03b-in-notin-parameterization.md)
- [Phase 3c: `hasSome`/`hasEvery` Parameterization](./03c-hassome-hasevery-parameterization.md)

---

## Test Commands Summary

```bash
# Build verification after each change
cargo build -p schema

# Unit tests
cargo test -p schema

# Update DMMF snapshots (REQUIRED after marking fields)
UPDATE_EXPECT=1 cargo test -p dmmf

# Verify snapshots pass
cargo test -p dmmf

# Full unit test suite
make test-unit
```

---

## Notes

- This phase will touch many files - consider doing it incrementally
- The DMMF snapshot tests will show which fields got marked - **review these carefully**
- After this phase, filter and data fields should show `isParameterizable: true` in DMMF
- We can be conservative initially and add more parameterizable fields later
- List fields are intentionally excluded and will be addressed in follow-up tasks
- Ensure snapshots are committed after updating with `UPDATE_EXPECT=1`