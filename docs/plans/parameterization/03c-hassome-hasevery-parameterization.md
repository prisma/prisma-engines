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
    List(PrismaListValue),      // Vec<PrismaValue>
    FieldRef(ScalarFieldRef),
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

## The Challenge

`ConditionListValue` currently has two variants:
1. `List(Vec<PrismaValue>)` - for literal list values
2. `FieldRef(ScalarFieldRef)` - for field references

Neither can represent a **placeholder for the entire list**. We need a way to pass through `PrismaValue::Placeholder` as the whole list value.

### Options

**Option A: Add `Placeholder` variant to `ConditionListValue`**
```rust
pub enum ConditionListValue {
    List(PrismaListValue),
    FieldRef(ScalarFieldRef),
    Placeholder(PrismaValue),  // New: entire list as placeholder
}
```

**Option B: Use single-element list with placeholder**
- Pass `ConditionListValue::List(vec![PrismaValue::Placeholder(...)])`
- Detect in SQL generation and handle specially

**Option C: Add Template variants (like `InTemplate`)**
```rust
pub enum ScalarListCondition {
    ContainsEveryTemplate(ConditionValue),  // New
    ContainsSomeTemplate(ConditionValue),   // New
    // ...
}
```

### Recommended: Option A

Option A is cleanest because:
1. Explicit representation of the placeholder case
2. No ambiguity (Option B could confuse single-placeholder with single-element list)
3. Less invasive than Option C (no new SQL generation branches needed)
4. SQL generation can simply use the placeholder value directly as an array parameter

---

## Implementation

### Task 3c.1: Add `Placeholder` Variant to `ConditionListValue`

**File:** `query-compiler/query-structure/src/filter/scalar/condition/value.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConditionListValue {
    List(PrismaListValue),
    FieldRef(ScalarFieldRef),
    Placeholder(PrismaValue),  // NEW
}

impl ConditionListValue {
    // Update existing methods...
    
    pub fn placeholder(pv: PrismaValue) -> Self {
        Self::Placeholder(pv)
    }
    
    pub fn len(&self) -> usize {
        match self {
            ConditionListValue::List(list) => list.len(),
            ConditionListValue::FieldRef(_) => 1,
            ConditionListValue::Placeholder(_) => 1,  // Unknown at compile time
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ConditionListValue::List(list) => list.is_empty(),
            _ => false,  // Can't know for FieldRef/Placeholder
        }
    }

    pub fn as_field_ref(&self) -> Option<&ScalarFieldRef> {
        if let Self::FieldRef(v) = self { Some(v) } else { None }
    }
    
    pub fn as_placeholder(&self) -> Option<&PrismaValue> {
        if let Self::Placeholder(v) = self { Some(v) } else { None }
    }
}
```

### Task 3c.2: Update Filter Parsing

**File:** `query-compiler/core/src/query_graph_builder/extractors/filters/scalar.rs`

Update `as_condition_list_value` to handle placeholders:

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
            Ok(ConditionListValue::Placeholder(PrismaValue::Placeholder(p)))
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

Add handling for `Placeholder` variant in `visit_scalar_list_filter`:

```rust
fn visit_scalar_list_filter(
    &mut self,
    filter: ScalarListFilter,
    // ...
) -> ConditionTree<'static> {
    let condition = match cond {
        // Existing: Contains with literal value
        ScalarListCondition::Contains(ConditionValue::Value(val)) => {
            comparable.compare_raw("@>", convert_list_pv(field, vec![val], ctx))
        }
        // Existing: Contains with field ref
        ScalarListCondition::Contains(ConditionValue::FieldRef(field_ref)) => {
            let field_ref_expr: Expression = field_ref.aliased_col(alias, ctx).into();
            field_ref_expr.equals(comparable.any())
        }
        
        // Existing: ContainsEvery with literal list
        ScalarListCondition::ContainsEvery(ConditionListValue::List(vals)) => {
            comparable.compare_raw("@>", convert_list_pv(field, vals, ctx))
        }
        // Existing: ContainsEvery with field ref
        ScalarListCondition::ContainsEvery(ConditionListValue::FieldRef(field_ref)) => {
            comparable.compare_raw("@>", field_ref.aliased_col(alias, ctx))
        }
        // NEW: ContainsEvery with placeholder
        ScalarListCondition::ContainsEvery(ConditionListValue::Placeholder(pv)) => {
            let param = convert_placeholder_to_array(field, pv, ctx);
            comparable.compare_raw("@>", param)
        }
        
        // Existing: ContainsSome with literal list
        ScalarListCondition::ContainsSome(ConditionListValue::List(vals)) => {
            comparable.compare_raw("&&", convert_list_pv(field, vals, ctx))
        }
        // Existing: ContainsSome with field ref
        ScalarListCondition::ContainsSome(ConditionListValue::FieldRef(field_ref)) => {
            comparable.compare_raw("&&", field_ref.aliased_col(alias, ctx))
        }
        // NEW: ContainsSome with placeholder
        ScalarListCondition::ContainsSome(ConditionListValue::Placeholder(pv)) => {
            let param = convert_placeholder_to_array(field, pv, ctx);
            comparable.compare_raw("&&", param)
        }
        
        // ... isEmpty cases ...
    };

    ConditionTree::single(condition)
}

/// Convert a placeholder PrismaValue to an array-typed Expression
fn convert_placeholder_to_array<'a>(
    field: &ScalarFieldRef,
    pv: PrismaValue,
    ctx: &Context<'_>,
) -> Expression<'a> {
    // The placeholder represents the entire array
    // Create a parameterized expression with the placeholder value
    Expression::from(field.value(pv, ctx))
}
```

### Task 3c.4: Verify `ScalarFieldExt::value()` Handles Placeholders

**File:** Check implementation location (likely in sql-query-builder or query-structure)

Ensure `ScalarFieldExt::value(pv, ctx)` can handle `PrismaValue::Placeholder`:

```rust
fn value(&self, pv: PrismaValue, ctx: &Context<'_>) -> Value<'static> {
    match pv {
        PrismaValue::Placeholder(p) => {
            // Return a placeholder Value that will be serialized correctly
            Value::placeholder(p.name, /* type info */)
        }
        // ... other cases ...
    }
}
```

### Task 3c.5: Mark Fields as Parameterizable

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

### Task 3c.6: Update Condition Inversion if Needed

**File:** `query-compiler/query-structure/src/filter/list.rs`

Ensure `ScalarListCondition` handles `Placeholder` variant in any inversion or transformation methods.

### Task 3c.7: Add Tests

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
- [ ] Add `Placeholder` variant to `ConditionListValue`
- [ ] Update `as_condition_list_value()` to handle placeholders
- [ ] Update `visit_scalar_list_filter()` for `ContainsEvery(Placeholder(...))`
- [ ] Update `visit_scalar_list_filter()` for `ContainsSome(Placeholder(...))`
- [ ] Verify/update `ScalarFieldExt::value()` for placeholders
- [ ] Mark `hasEvery` as `.parameterizable()`
- [ ] Mark `hasSome` as `.parameterizable()`

### Verification
- [ ] `ConditionListValue::Placeholder` serializes correctly
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

**Medium Risk**

### Advantages over `in`/`notIn`
1. **No dynamic SQL expansion** - PostgreSQL handles arrays natively
2. **Single parameter** - No `ParameterizedRow` complexity
3. **Fixed SQL structure** - `col && $1` regardless of array size

### Potential Issues
1. **New enum variant** - Need to update all pattern matches on `ConditionListValue`
2. **Type handling** - Placeholder must be list-typed, array element types must match
3. **Database support** - Only PostgreSQL/CockroachDB have `ScalarLists` capability
4. **Empty array semantics** - Different behavior for `hasSome` vs `hasEvery`

### Mitigation
1. Compiler will catch missing pattern matches when adding new variant
2. Add type validation in parser
3. Feature is already gated by capability checks
4. Document empty array behavior clearly in tests

---

## Dependencies

- Phase 1 (Schema Infrastructure) - `is_parameterizable` flag exists
- Phase 2 (DMMF Output) - flag exposed in DMMF
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
