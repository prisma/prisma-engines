# Phase 3c: `hasSome`/`hasEvery` Filter Parameterization

## Objective

Enable parameterization of the `hasSome` and `hasEvery` filter fields used on scalar list fields, allowing queries like:
```typescript
prisma.post.findMany({ where: { tags: { hasSome: $searchTags } } })
prisma.post.findMany({ where: { tags: { hasEvery: $requiredTags } } })
```

Where `$searchTags` and `$requiredTags` are placeholders representing entire arrays.

## Background

### What Are `hasSome`/`hasEvery`?

These filters operate on **scalar list fields** (e.g., `tags String[]`):

- **`hasSome`**: Returns records where the list field contains **at least one** of the provided values
- **`hasEvery`**: Returns records where the list field contains **all** of the provided values

### SQL Generation (PostgreSQL)

```sql
-- hasSome: Array overlap operator
SELECT * FROM posts WHERE tags && ARRAY['tag1', 'tag2']

-- hasEvery: Array contains operator  
SELECT * FROM posts WHERE tags @> ARRAY['tag1', 'tag2']
```

### Key Difference from `in`/`notIn`

Unlike `in`/`notIn` which require `ParameterizedRow` for dynamic SQL expansion, PostgreSQL **natively supports array parameters**:

```sql
-- in/notIn: SQL structure changes with list length
WHERE id IN ($1, $2, $3)     -- 3 placeholders needed

-- hasSome/hasEvery: Single array parameter regardless of length
WHERE tags && $1             -- $1 is the entire array
WHERE tags @> $1             -- $1 is the entire array
```

This means `hasSome`/`hasEvery` can use regular `ExpressionKind::Parameterized` with an array value - **no `ParameterizedRow` needed**.

### Current Implementation

**Schema definition** (`field_filter_types.rs`):
```rust
// In scalar_list_filter_type()
let mapped_list_type_with_field_ref_input = mapped_list_type.with_field_ref_input();
fields.push(input_field(filters::HAS_EVERY, mapped_list_type_with_field_ref_input.clone(), None).optional());
fields.push(input_field(filters::HAS_SOME, mapped_list_type_with_field_ref_input, None).optional());
```

**Query graph builder** (`scalar.rs`):
```rust
filters::HAS_EVERY => Ok(vec![field.contains_every_element(self.as_condition_list_value(input)?)])
filters::HAS_SOME => Ok(vec![field.contains_some_element(self.as_condition_list_value(input)?)])
```

**Filter structure** (`query-structure/src/filter/list.rs`):
```rust
pub enum ScalarListCondition {
    ContainsEvery(ConditionListValue),  // hasEvery
    ContainsSome(ConditionListValue),   // hasSome
    // ...
}

pub enum ConditionListValue {
    List(PrismaListValue),              // Vec<PrismaValue>
    FieldRef(ScalarFieldRef),
    Placeholder(prisma_value::Placeholder),  // Added in Phase 3b refactoring
}
```

**SQL generation** (`sql-query-builder/src/filter/visitor.rs`):
```rust
ScalarListCondition::ContainsEvery(ConditionListValue::List(vals)) => {
    comparable.compare_raw("@>", convert_list_pv(field, vals, ctx))
}
ScalarListCondition::ContainsSome(ConditionListValue::List(vals)) => {
    comparable.compare_raw("&&", convert_list_pv(field, vals, ctx))
}
```

---

## Prerequisites (Completed in Phase 3b)

As part of the Phase 3b refactoring that consolidated `InTemplate`/`NotInTemplate` into `ConditionListValue::Placeholder`, the following is already done:

✅ **`ConditionListValue::Placeholder` variant exists:**
```rust
pub enum ConditionListValue {
    List(PrismaListValue),
    FieldRef(ScalarFieldRef),
    Placeholder(prisma_value::Placeholder),  // Already added!
}
```

✅ **Stub implementations in SQL visitor** - `unimplemented!()` stubs for `ContainsEvery(Placeholder(_))` and `ContainsSome(Placeholder(_))`

✅ **MongoDB connector stubs** - `unimplemented!()` for placeholder variants

What remains is:
1. Update filter parsing to create `ConditionListValue::Placeholder` for `hasSome`/`hasEvery`
2. Implement actual SQL generation (replace `unimplemented!()` stubs)
3. Mark fields as parameterizable in schema

---

## Implementation

### Task 3c.1: ~~Add `Placeholder` Variant to `ConditionListValue`~~ ✅ DONE

**Completed in Phase 3b refactoring.** The variant uses `prisma_value::Placeholder` directly (not `PrismaValue`) for better type safety.

### Task 3c.2: Update Filter Parsing

**File:** `query-compiler/core/src/query_graph_builder/extractors/filters/scalar.rs`

Update `as_condition_list_value` to handle placeholders. Extract the inner `Placeholder` from `PrismaValue::Placeholder`:

```rust
fn as_condition_list_value(&self, input: ParsedInputValue<'_>) -> QueryGraphBuilderResult<ConditionListValue> {
    let field = self.field();

    match input {
        // Field reference case (existing)
        ParsedInputValue::Map(mut map) => {
            // ... existing field ref handling ...
        }
        
        // NEW: Handle placeholder for entire list
        ParsedInputValue::Single(PrismaValue::Placeholder(p)) => {
            Ok(ConditionListValue::Placeholder(p))  // Use inner Placeholder directly
        }
        
        // Literal list case (existing)
        _ => {
            let vals: Vec<PrismaValue> = input.try_into()?;
            Ok(ConditionListValue::list(vals))
        }
    }
}
```

### Task 3c.3: Update SQL Generation

**File:** `query-compiler/query-builders/sql-query-builder/src/filter/visitor.rs`

Replace the `unimplemented!()` stubs (added in Phase 3b) with actual implementations:

```rust
// Current stub (to be replaced):
ScalarListCondition::ContainsEvery(ConditionListValue::Placeholder(_)) => {
    unimplemented!("Placeholder support for hasSome/hasEvery not yet implemented")
}
ScalarListCondition::ContainsSome(ConditionListValue::Placeholder(_)) => {
    unimplemented!("Placeholder support for hasSome/hasEvery not yet implemented")
}

// Replace with:
ScalarListCondition::ContainsEvery(ConditionListValue::Placeholder(placeholder)) => {
    let param = convert_placeholder_to_array(field, placeholder, ctx);
    comparable.compare_raw("@>", param)
}
ScalarListCondition::ContainsSome(ConditionListValue::Placeholder(placeholder)) => {
    let param = convert_placeholder_to_array(field, placeholder, ctx);
    comparable.compare_raw("&&", param)
}

/// Convert a Placeholder to an array-typed Expression
fn convert_placeholder_to_array<'a>(
    field: &ScalarFieldRef,
    placeholder: prisma_value::Placeholder,
    ctx: &Context<'_>,
) -> Expression<'a> {
    // Wrap the Placeholder back into PrismaValue for the field.value() call
    Expression::from(field.value(placeholder.into(), ctx))
}
```

Note: The `Placeholder` here is `prisma_value::Placeholder`, not `PrismaValue`. We wrap it back into `PrismaValue::Placeholder` when calling `field.value()`.

### Task 3c.4: ~~Verify `ScalarFieldExt::value()` Handles Placeholders~~ ✅ Already Verified

**File:** `query-compiler/query-builders/sql-query-builder/src/model_extensions/scalar_field.rs`

This is already implemented and used by `in`/`notIn` parameterization:

```rust
(PrismaValue::Placeholder(PrismaValuePlaceholder { name, .. }), ident) => {
    Value::opaque(Placeholder::new(name), convert::type_identifier_to_opaque_type(&ident))
}
```

### Task 3c.5: Mark Fields as Parameterizable (TODO)

**File:** `query-compiler/schema/src/build/input_types/fields/field_filter_types.rs`

```rust
// Before:
fields.push(input_field(filters::HAS_EVERY, mapped_list_type_with_field_ref_input.clone(), None).optional());
fields.push(input_field(filters::HAS_SOME, mapped_list_type_with_field_ref_input, None).optional());

// After:
fields.push(input_field(filters::HAS_EVERY, mapped_list_type_with_field_ref_input.clone(), None)
    .optional()
    .parameterizable());
fields.push(input_field(filters::HAS_SOME, mapped_list_type_with_field_ref_input, None)
    .optional()
    .parameterizable());
```

### Task 3c.6: ~~Update Condition Inversion if Needed~~ ✅ Not Required

**File:** `query-compiler/query-structure/src/filter/list.rs`

`ScalarListCondition` doesn't have an `invert()` method like `ScalarCondition` does. The NOT modifier is handled at the filter tree level, not the condition level. No changes needed.

### Task 3c.7: Update MongoDB Connector (TODO)

**File:** `query-engine/connectors/mongodb-query-connector/src/filter.rs`

Replace the `unimplemented!()` stubs (added in Phase 3b) when MongoDB QC support is implemented:

```rust
// Current stubs:
ScalarListCondition::ContainsEvery(ConditionListValue::Placeholder(_)) => {
    unimplemented!("query compiler not supported with mongodb yet")
}
ScalarListCondition::ContainsSome(ConditionListValue::Placeholder(_)) => {
    unimplemented!("query compiler not supported with mongodb yet")
}
```

### Task 3c.8: Add Tests

**Integration tests:**

1. **Basic `hasSome` with placeholder:**
   ```typescript
   findMany({ where: { tags: { hasSome: $searchTags } } })
   ```

2. **Basic `hasEvery` with placeholder:**
   ```typescript
   findMany({ where: { tags: { hasEvery: $requiredTags } } })
   ```

3. **Empty array handling:**
   - `hasSome` with empty array → should return no results
   - `hasEvery` with empty array → should return all results (vacuous truth)

4. **Combined with other filters:**
   ```typescript
   findMany({ where: { tags: { hasSome: $tags }, status: { equals: $status } } })
   ```

5. **NOT modifier:**
   ```typescript
   findMany({ where: { NOT: { tags: { hasSome: $excludeTags } } } })
   ```

---

## Data Flow Summary

```
User Query:
  where: { tags: { hasSome: placeholder("tags", StringList) } }
                    ↓
Parser:
  ParsedInputValue::Single(PrismaValue::Placeholder("tags", StringList))
                    ↓
Filter Extractor (scalar.rs):
  as_condition_list_value() → ConditionListValue::Placeholder(PrismaValue::Placeholder(...))
  field.contains_some_element(value)
                    ↓
Filter Structure:
  ScalarListCondition::ContainsSome(ConditionListValue::Placeholder(...))
                    ↓
SQL Builder (visitor.rs):
  convert_placeholder_to_array() → Expression::Parameterized(Value::placeholder(...))
  comparable.compare_raw("&&", param)
                    ↓
Quaint Visitor:
  visit_parameterized() → write $N placeholder
                    ↓
Query Template:
  "SELECT ... WHERE tags && $1"
  parameters: [placeholder("tags")]
                    ↓
Query Interpreter in Prisma Client (execution time):
  Replaces $1 with actual array value
                    ↓
Final SQL:
  "SELECT ... WHERE tags && $1"
  params: ['tag1', 'tag2', 'tag3']  (as PostgreSQL array)
```

---

## Verification Checklist

### Code Changes
- [x] Add `Placeholder` variant to `ConditionListValue` *(Done in Phase 3b)*
- [x] Add stub implementations in SQL visitor *(Done in Phase 3b)*
- [x] Add stub implementations in MongoDB connector *(Done in Phase 3b)*
- [x] Verify `ScalarFieldExt::value()` handles placeholders *(Already works)*
- [ ] Update `as_condition_list_value()` to handle placeholders
- [ ] Replace SQL visitor stubs with actual `ContainsEvery(Placeholder(...))` implementation
- [ ] Replace SQL visitor stubs with actual `ContainsSome(Placeholder(...))` implementation
- [ ] Mark `hasEvery` as `.parameterizable()`
- [ ] Mark `hasSome` as `.parameterizable()`

### Verification
- [x] `ConditionListValue::Placeholder` type exists and compiles
- [ ] SQL generation produces `col && $1` / `col @> $1`
- [ ] PostgreSQL receives array as single parameter
- [ ] Empty array edge cases handled correctly

### Tests
- [ ] Unit test: `ContainsSome(Placeholder(...))` SQL generation
- [ ] Unit test: `ContainsEvery(Placeholder(...))` SQL generation
- [ ] Integration test: `hasSome` with placeholder
- [ ] Integration test: `hasEvery` with placeholder
- [ ] Integration test: Empty array placeholder
- [ ] Integration test: Combined filters
- [ ] DMMF snapshot shows `isParameterizable: true`

---

## Risk Assessment

**Low Risk** (reduced from Medium due to Phase 3b groundwork)

### Advantages over `in`/`notIn`
1. **No dynamic SQL expansion** - PostgreSQL handles arrays natively
2. **Single parameter** - No `ParameterizedRow` complexity
3. **Fixed SQL structure** - `col && $1` regardless of array size

### Potential Issues
1. ~~**New enum variant** - Need to update all pattern matches on `ConditionListValue`~~ ✅ Done in Phase 3b
2. **Type handling** - Placeholder must be list-typed, array element types must match
3. **Database support** - Only PostgreSQL/CockroachDB have `ScalarLists` capability
4. **Empty array semantics** - Different behavior for `hasSome` vs `hasEvery`

### Mitigation
1. ~~Compiler will catch missing pattern matches when adding new variant~~ ✅ Already addressed
2. Add type validation in parser
3. Feature is already gated by capability checks
4. Document empty array behavior clearly in tests

---

## Dependencies

- Phase 1 (Schema Infrastructure) - `is_parameterizable` flag exists ✅
- Phase 2 (DMMF Output) - flag exposed in DMMF ✅
- Phase 3b (`in`/`notIn` Parameterization) - `ConditionListValue::Placeholder` variant added ✅
- Phase 4 (Parser Validation) - validates placeholder usage

## Related Code

- `ConditionListValue` defined in: `query-structure/src/filter/scalar/condition/value.rs`
- `ScalarListCondition` defined in: `query-structure/src/filter/list.rs`
- SQL visitor in: `sql-query-builder/src/filter/visitor.rs`
- Schema fields in: `schema/src/build/input_types/fields/field_filter_types.rs`

## Notes

- Only applicable to databases with `ScalarLists` capability (PostgreSQL, CockroachDB)
- This is lower priority than `in`/`notIn` since scalar lists are less commonly used
- Consider implementing after `in`/`notIn` to benefit from learnings
- The same approach could potentially work for `equals` on scalar list fields
