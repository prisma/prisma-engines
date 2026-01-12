# Phase 3b: `in`/`notIn` Filter Parameterization

## Objective

Enable parameterization of the `in` and `notIn` filter fields, allowing queries like:
```typescript
prisma.user.findMany({ where: { id: { in: $ids } } })
```

Where `$ids` is a placeholder representing an entire list of values.

## Background

### How `in`/`notIn` Differs from Other Filters

Unlike scalar filters (`equals`, `contains`, etc.), `in`/`notIn` accept a **list of values**. This creates a challenge for SQL generation:

```sql
-- The number of placeholders varies with list length
SELECT * FROM users WHERE id IN ($1, $2, $3)      -- 3 values
SELECT * FROM users WHERE id IN ($1, $2, $3, $4)  -- 4 values
```

The SQL structure itself changes based on the list length, which is why we need special handling.

### Current Implementation

**Schema definition** (`field_filter_types.rs`):
```rust
// In inclusion_filters()
input_field(filters::IN, field_types.clone(), None)
    .optional()
    .nullable_if(nullable)
```

**Query graph builder** (`scalar.rs` lines ~85-103):
```rust
filters::IN => {
    let value = self.as_condition_value(input, true)?;

    let filter = match value {
        ConditionValue::Value(value) => match value {
            PrismaValue::Null if self.reverse() => field.not_equals(value),
            PrismaValue::List(values) if self.reverse() => field.not_in(values),
            PrismaValue::Null => field.equals(value),
            PrismaValue::List(values) => field.is_in(values),
            _ => unreachable!(), // <-- BUG: Placeholder hits this!
        },
        ConditionValue::FieldRef(field_ref) => field.is_in(field_ref),
    };
    Ok(vec![filter])
}
```

**Filter structure** (`query-structure/src/filter/scalar/condition/mod.rs`):
```rust
pub enum ScalarCondition {
    // Regular in/notIn - takes a list
    In(ConditionListValue),
    NotIn(ConditionListValue),
    
    // Template variants - takes a single value representing the whole list
    InTemplate(ConditionValue),      // <-- Used for placeholders!
    NotInTemplate(ConditionValue),
    // ...
}
```

**SQL generation for `InTemplate`** (`sql-query-builder/src/filter/visitor.rs`):
```rust
ScalarCondition::InTemplate(ConditionValue::Value(value)) => {
    let sql_value = convert_first_value(fields, value, alias, ctx);
    comparable.in_selection(sql_value.into_parameterized_row())  // Uses ParameterizedRow!
}
```

### The `ParameterizedRow` Mechanism

`ParameterizedRow` is the key to handling variable-length lists:

1. **At query build time**: Generates `Fragment::ParameterTuple` in the query template
2. **At execution time**: The query interpreter expands `[$1]` to `($1, $2, ..., $N)` based on actual array length
3. **SQL output**: `col IN [$1]` becomes `col IN ($1, $2, $3)` when given 3 values

This allows a single cached query plan to work with lists of any length.

### Existing `InTemplate` Usage - Already in Production!

`InTemplate` is **actively used in production** for join conditions in every query involving relations:

1. **In `add_inmemory_join`** (`query-compiler/src/translate/query/read.rs:167-168`):
   When building joins for related records with non-unique parents:
   ```rust
   let condition = if has_unique_parent {
       ScalarCondition::Equals(ConditionValue::value(placeholder))
   } else {
       ScalarCondition::InTemplate(ConditionValue::value(placeholder))
   };
   ```

2. **In `build_read_related_records`** (`query-compiler/src/translate/query/read.rs:233-235`):
   When processing parent results for relation queries:
   ```rust
   linkage.add_condition(sf.clone(), ScalarCondition::InTemplate(val.into()));
   ```

3. **In `IntoFilter` for `Vec<SelectionResult>`** (`query-structure/src/filter/into_filter.rs:37-40`):
   When converting selection results with placeholders to filters:
   ```rust
   acc.push(sf.is_in_template(val.clone()));
   ```

This means the entire `InTemplate` → `ParameterizedRow` → `Fragment::ParameterTuple` code path is **already well-exercised in production** for every query involving relations with multiple parent records.

---

## Implementation

### Task 3b.1: Fix Filter Parsing for Placeholders

**File:** `query-compiler/core/src/query_graph_builder/extractors/filters/scalar.rs`

The current code hits `unreachable!()` when given a placeholder. We need to handle placeholders by creating `InTemplate`/`NotInTemplate`:

```rust
filters::IN => {
    let value = self.as_condition_value(input, true)?;

    let filter = match value {
        ConditionValue::Value(value) => match value {
            PrismaValue::Null if self.reverse() => field.not_equals(value),
            PrismaValue::List(values) if self.reverse() => field.not_in(values),
            PrismaValue::Null => field.equals(value),
            PrismaValue::List(values) => field.is_in(values),
            
            // NEW: Handle placeholders - use Template variants
            pv @ PrismaValue::Placeholder(_) if self.reverse() => field.not_in_template(pv),
            pv @ PrismaValue::Placeholder(_) => field.is_in_template(pv),
            
            _ => unreachable!(), // Now only truly unreachable cases
        },
        ConditionValue::FieldRef(field_ref) if self.reverse() => field.not_in(field_ref),
        ConditionValue::FieldRef(field_ref) => field.is_in(field_ref),
    };
    Ok(vec![filter])
}
```

Do the same for `filters::NOT_IN`.

### Task 3b.2: Mark `in`/`notIn` as Parameterizable

**File:** `query-compiler/schema/src/build/input_types/fields/field_filter_types.rs`

**Location:** `inclusion_filters()` function

```rust
// Before:
vec![
    input_field(filters::IN, field_types.clone(), None)
        .optional()
        .nullable_if(nullable),
    input_field(filters::NOT_IN, field_types, None)
        .optional()
        .nullable_if(nullable),
]

// After:
vec![
    input_field(filters::IN, field_types.clone(), None)
        .optional()
        .nullable_if(nullable)
        .parameterizable(),
    input_field(filters::NOT_IN, field_types, None)
        .optional()
        .nullable_if(nullable)
        .parameterizable(),
]
```

### Task 3b.3: Verify `InTemplate` SQL Generation

**File:** `query-compiler/query-builders/sql-query-builder/src/filter/visitor.rs`

The code already exists but may not be fully tested:

```rust
ScalarCondition::InTemplate(ConditionValue::Value(value)) => {
    let sql_value = convert_first_value(fields, value, alias, ctx);
    comparable.in_selection(sql_value.into_parameterized_row())
}
ScalarCondition::InTemplate(ConditionValue::FieldRef(_)) => todo!(),  // May need implementation
ScalarCondition::NotInTemplate(ConditionValue::Value(value)) => {
    let sql_value = convert_first_value(fields, value, alias, ctx);
    comparable.not_in_selection(sql_value.into_parameterized_row())
}
ScalarCondition::NotInTemplate(ConditionValue::FieldRef(_)) => todo!(),
```

**Verify `convert_first_value` handles placeholders:**

```rust
fn convert_first_value<'a>(
    fields: &[ScalarFieldRef],
    value: PrismaValue,
    alias: Option<&str>,
    ctx: &Context<'_>,
) -> Expression<'a> {
    // Does this handle PrismaValue::Placeholder correctly?
    // Check field.value() implementation
}
```

### Task 3b.4: Verify `ParameterizedRow` Flow

**File:** `quaint/src/visitor.rs`

Verify the `ParameterizedRow` generates correct SQL:

```rust
fn visit_compare(&mut self, compare: Compare<'a>) -> Result {
    // ...
    Compare::In(left, right) => match (*left, *right) {
        // ...
        (
            left,
            Expression {
                kind: ExpressionKind::ParameterizedRow(value),
                ..
            },
        ) => {
            self.visit_expression(left)?;
            self.write(" IN ")?;
            self.visit_parameterized_row(value)  // Writes Fragment::ParameterTuple
        }
        // ...
    }
}
```

**File:** `quaint/src/visitor/postgres.rs` (and other DB visitors)

```rust
fn visit_parameterized_row(&mut self, value: Value<'a>) -> visitor::Result {
    self.query_template.write_parameter_tuple();  // Writes Fragment::ParameterTuple
    self.query_template.parameters.push(value);
    Ok(())
}
```

### Task 3b.5: Verify Query Template Handling

**File:** `libs/query-template/src/fragment.rs`

```rust
pub enum Fragment {
    StringChunk { chunk: String },
    Parameter,
    ParameterTuple,  // <-- For variable-length lists
    ParameterTupleList { ... },
}
```

Verify that `ParameterTuple` is correctly expanded by the query interpreter in the context of user-provided `in` clauses. Previously it was only used with automatically generated `IN` clauses for fetching child records in application-level joins.

### Task 3b.6: Handle Case-Insensitive Mode

**File:** `query-compiler/query-builders/sql-query-builder/src/filter/visitor.rs`

Check `insensitive_scalar_filter` function for `InTemplate`:

```rust
ScalarCondition::InTemplate(ConditionValue::Value(value)) => {
    let comparable = Expression::from(lower(comparable));
    let sql_value = convert_first_value(fields, value, alias, ctx);
    comparable.in_selection(sql_value.into_parameterized_row())
}
```

Ensure lowercase transformation works correctly with placeholders.

### Task 3b.7: Add Comprehensive Tests

**Integration tests needed:**

1. **Basic `in` with placeholder:**
   ```typescript
   findMany({ where: { id: { in: $ids } } })
   ```

2. **Basic `notIn` with placeholder:**
   ```typescript
   findMany({ where: { status: { notIn: $statuses } } })
   ```

3. **Empty list handling:**
   - What happens when placeholder resolves to `[]`?
   - `IN ()` is invalid SQL - should return no results

4. **Case-insensitive mode:**
   ```typescript
   findMany({ where: { email: { in: $emails, mode: 'insensitive' } } })
   ```

5. **Combined with other filters:**
   ```typescript
   findMany({ where: { id: { in: $ids }, name: { equals: $name } } })
   ```

6. **Nested filters:**
   ```typescript
   findMany({ where: { posts: { some: { status: { in: $statuses } } } } })
   ```

---

## Data Flow Summary

```
User Query:
  where: { id: { in: placeholder("ids", IntList) } }
                    ↓
Parser:
  ParsedInputValue::Single(PrismaValue::Placeholder("ids", IntList))
                    ↓
Filter Extractor (scalar.rs):
  as_condition_value() → ConditionValue::Value(PrismaValue::Placeholder(...))
  match: PrismaValue::Placeholder → field.is_in_template(pv)
                    ↓
Filter Structure:
  ScalarCondition::InTemplate(ConditionValue::Value(PrismaValue::Placeholder(...)))
                    ↓
SQL Builder (visitor.rs):
  convert_first_value() → Expression::Parameterized(Value::placeholder(...))
  .into_parameterized_row() → Expression::ParameterizedRow(Value::placeholder(...))
                    ↓
Quaint Visitor:
  visit_parameterized_row() → write Fragment::ParameterTuple
                    ↓
Query Template:
  "SELECT ... WHERE id IN [$1]"
  parameters: [placeholder("ids")]
                    ↓
Query Interpreter in Prisma Client (execution time):
  Expands [$1] to ($1, $2, $3) based on actual array length
                    ↓
Final SQL:
  "SELECT ... WHERE id IN ($1, $2, $3)"
  params: [1, 2, 3]
```

---

## Verification Checklist

### Code Changes
- [ ] `scalar.rs`: Handle `PrismaValue::Placeholder` in `filters::IN` branch
- [ ] `scalar.rs`: Handle `PrismaValue::Placeholder` in `filters::NOT_IN` branch
- [ ] `field_filter_types.rs`: Mark `in` field as `.parameterizable()`
- [ ] `field_filter_types.rs`: Mark `notIn` field as `.parameterizable()`

### Verification
- [ ] `convert_first_value` handles `PrismaValue::Placeholder` correctly
- [ ] `ScalarFieldExt::value()` handles `PrismaValue::Placeholder`
- [ ] `into_parameterized_row()` works with placeholder values
- [ ] `Fragment::ParameterTuple` is correctly serialized in query template
- [ ] Query Interpreter correctly expands `ParameterTuple` at execution time
- [ ] Empty list case is handled (should return no results)

### Tests
- [ ] Unit test: `InTemplate` SQL generation
- [ ] Unit test: `NotInTemplate` SQL generation
- [ ] Integration test: `in` with placeholder
- [ ] Integration test: `notIn` with placeholder
- [ ] Integration test: Empty list placeholder
- [ ] Integration test: Case-insensitive mode with placeholder
- [ ] DMMF snapshot shows `isParameterizable: true` for `in`/`notIn`

---

## Risk Assessment

**Low-Medium Risk** - Lower than initially expected because:

1. **Well-tested code path**: `InTemplate`/`NotInTemplate` are already used in production for join conditions on every relation query with multiple parent records
2. **`ParameterizedRow` proven**: The entire `InTemplate` → `ParameterizedRow` → `Fragment::ParameterTuple` chain is exercised daily in production
3. **Query interpreter handles expansion**: The TypeScript query interpreter already correctly expands `ParameterTuple` fragments

### Remaining Considerations

1. **Filter parsing change**: The main change is in `scalar.rs` to route placeholders to `InTemplate` instead of hitting `unreachable!()`
2. **Empty list produces invalid SQL** - `IN ()` is invalid; but this is already handled!
   The `QueryInterpreter` already handles this (see [`render-query.ts`](https://github.com/prisma/prisma/blob/c23ba8b4f9d620753ad59668a42b44a0651002ac/packages/client-engine-runtime/src/interpreter/render-query.ts#L128-L134)):
   ```typescript
   case 'parameterTuple': {
     const placeholders =
       fragment.value.length == 0
         ? 'NULL'
         : fragment.value.map(() => formatPlaceholder(placeholderFormat, ctx.placeholderNumber++)).join(',')
     return `(${placeholders})`
   }
   ```
3. **Type coercion** - Placeholder type must match list element type
4. **Insensitive mode** - `LOWER()` transformation with placeholders (code exists, needs verification)

### What's Actually New

The only new code path is in `scalar.rs` - routing `PrismaValue::Placeholder` to `is_in_template()` instead of hitting the `unreachable!()` branch. Everything downstream is already production-tested.

---

## Dependencies

- Phase 1 (Schema Infrastructure) - `is_parameterizable` flag exists
- Phase 2 (DMMF Output) - flag exposed in DMMF  
- Phase 4 (Parser Validation) - validates placeholder usage

## Related Code

- `InTemplate` created in: `query-compiler/src/translate/query/read.rs` (join conditions)
- `ParameterizedRow` defined in: `quaint/src/ast/expression.rs`
- `Fragment::ParameterTuple` defined in: `libs/query-template/src/fragment.rs`
- Visitor handling in: `quaint/src/visitor.rs`

## Notes

- This is higher priority than `hasSome`/`hasEvery` since `in`/`notIn` are used more frequently
- The `InTemplate` infrastructure was added for join conditions, repurposing for user queries
- Consider adding query template tests that specifically verify `ParameterTuple` expansion
