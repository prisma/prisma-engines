# Phase 4: Query Parser Validation

## Objective

Update the query document parser to validate that placeholder values (`PrismaValue::Placeholder`) are only used in fields marked as parameterizable. Return a clear validation error when a placeholder is used in a non-parameterizable context.

## Files to Modify

1. `query-compiler/core/src/query_document/parser.rs`
2. `libs/user-facing-errors/src/query_engine/validation.rs`

---

## Background

### Current Behavior (lines ~256-261 in parser.rs)

```rust
// TODO: make query parsing aware of whether we are using the query compiler,
// and disallow placeholders and generator calls in the query document if we are not.
if let ArgumentValue::Scalar(pv @ PrismaValue::Placeholder { .. }) = &value {
    return Ok(ParsedInputValue::Single(pv.clone()));
}
if let ArgumentValue::Scalar(pv @ PrismaValue::GeneratorCall { .. }) = &value {
    return Ok(ParsedInputValue::Single(pv.clone()));
}
```

Currently, placeholders are accepted unconditionally without validation.

> **Note:** The TODO comment is outdated and should be removed. It refers to disallowing placeholders in the old query engine (which no longer exists) vs. query compiler. Since the query engine has been removed and query compiler is now the only code path, this comment is no longer relevant. Our task is different: we're adding validation to reject placeholders in *non-parameterizable fields*, not rejecting them entirely.

### Desired Behavior

1. Check if the current field is marked as parameterizable
2. If parameterizable: accept the placeholder (current behavior)
3. If NOT parameterizable: return a validation error with clear message

---

## Task 4.1: Add New Validation Error Type

**File:** `libs/user-facing-errors/src/query_engine/validation.rs`

### Locate the error enum/functions

Search for existing validation errors to understand the pattern:

```bash
grep -n "pub fn.*error\|ValidationError" libs/user-facing-errors/src/query_engine/validation.rs | head -30
```

### Add new error variant/function

Add a new function to create a "placeholder not allowed" error:

```rust
/// Error returned when a placeholder is used in a field that doesn't support parameterization.
pub fn placeholder_not_allowed(
    selection_path: &[String],
    argument_path: &[String],
    field_name: &str,
) -> Self {
    // Follow the pattern of existing validation errors
    // The error message should clearly explain:
    // 1. What field the placeholder was used in
    // 2. That this field doesn't support placeholders
    // 3. Which fields typically DO support placeholders (filters, data values)
    
    Self::new(
        // ... construct error with appropriate fields
    )
}
```

### Example Error Message

```
Placeholder not allowed in field `take`.

The field `take` does not support query parameter placeholders because it affects 
the structure of the query plan. Placeholders can only be used in fields that accept 
user data values, such as filter conditions (where, equals, contains, etc.) and 
data fields in create/update operations.

Selection path: Mutation.updateUser
Argument path: take
```

---

## Task 4.2: Update Parser to Validate Placeholders

**File:** `query-compiler/core/src/query_document/parser.rs`

### Locate the parse_input_value function

The validation needs to happen in `parse_input_value` which is called with:
- `selection_path`: Path to the current selection (e.g., `Query.findManyUser`)
- `argument_path`: Path to the current argument (e.g., `where.id.equals`)
- `value`: The input value being parsed
- `possible_input_types`: The allowed types for this position

### Challenge: Accessing Field Information

The current `parse_input_value` signature doesn't have direct access to the `InputField`. We need to either:

**Option A**: Pass `InputField` (or just `is_parameterizable`) to `parse_input_value`

**Option B**: Check parameterizability at a higher level before calling `parse_input_value`

**Option C**: Add `is_parameterizable` to the context/path information

### Recommended Approach: Option A

Modify the function signature to accept parameterizability info:

```rust
fn parse_input_value<'a>(
    &self,
    selection_path: SelectionPath<'_>,
    argument_path: ArgumentPath<'_>,
    value: ArgumentValue,
    possible_input_types: &[InputType<'a>],
    query_schema: &'a QuerySchema,
    is_parameterizable: bool,  // NEW parameter
) -> QueryParserResult<ParsedInputValue<'a>> {
```

### Updated Validation Logic

```rust
// Near line ~256, replace:
if let ArgumentValue::Scalar(pv @ PrismaValue::Placeholder { .. }) = &value {
    return Ok(ParsedInputValue::Single(pv.clone()));
}

// With:
if let ArgumentValue::Scalar(pv @ PrismaValue::Placeholder { .. }) = &value {
    if !is_parameterizable {
        return Err(ValidationError::placeholder_not_allowed(
            selection_path.segments(),
            argument_path.segments(),
            argument_path.last().unwrap_or_default(),
        ));
    }
    return Ok(ParsedInputValue::Single(pv.clone()));
}
```

### Handle GeneratorCall Similarly

```rust
if let ArgumentValue::Scalar(pv @ PrismaValue::GeneratorCall { .. }) = &value {
    if !is_parameterizable {
        return Err(ValidationError::placeholder_not_allowed(
            selection_path.segments(),
            argument_path.segments(),
            argument_path.last().unwrap_or_default(),
        ));
    }
    return Ok(ParsedInputValue::Single(pv.clone()));
}
```

---

## Task 4.3: Update Call Sites

After modifying `parse_input_value` signature, update all call sites to pass `is_parameterizable`.

### Search for call sites:

```bash
grep -n "parse_input_value" query-compiler/core/src/query_document/parser.rs
```

### Key call sites to update:

1. **`parse_input_field`** - This is where we have access to `InputField`:

```rust
fn parse_input_field<'a>(
    &self,
    selection_path: SelectionPath<'_>,
    argument_path: ArgumentPath<'_>,
    input_field: &InputField<'a>,
    value: ArgumentValue,
    query_schema: &'a QuerySchema,
) -> QueryParserResult<ParsedInputValue<'a>> {
    self.parse_input_value(
        selection_path,
        argument_path,
        value,
        input_field.field_types(),
        query_schema,
        input_field.is_parameterizable(),  // NEW: pass the flag
    )
}
```

2. **Recursive calls for lists** - When parsing a list, the elements inherit the parameterizability of the list field itself.

3. **Object field parsing** - When parsing object fields, each nested field gets its `is_parameterizable` from its own `InputField` definition (no inheritance from parent).

4. **Other entry points** - Check if there are other functions that call `parse_input_value` directly.

---

## Task 4.4: Handle Nested Objects (No Inheritance)

> **Important:** There is NO inheritance or propagation of `is_parameterizable`. Each `InputField` has its own flag set at schema build time. When parsing nested objects, each field lookup retrieves a fresh `InputField` with its own `is_parameterizable` value.

When parsing nested objects (e.g., `where: { id: { equals: 5 } }`), parameterizability is determined by each field's own `InputField` definition:

**Example traversal:**
1. `where` field is parsed → it's an object wrapper, `is_parameterizable: false` (but doesn't matter - not a scalar)
2. Object fields are parsed → `id` field looked up from `WhereInput` type
3. `id` field is parsed → it's an object wrapper (filter), `is_parameterizable: false` (but doesn't matter - not a scalar)
4. Filter fields are parsed → `equals` field looked up from `IntFilter` type
5. `equals` field is parsed → **scalar value**, `is_parameterizable: true` from `equals` field's `InputField` ✓

The key insight: **only scalar values can be placeholders**, and we only check `is_parameterizable` when we encounter a scalar `ArgumentValue::Scalar(PrismaValue::Placeholder { .. })`. Object wrappers don't need the flag because they can never contain a placeholder directly.

### How the parsing naturally works:

```rust
// When parsing object fields (around line ~370-400):
(ArgumentValue::Object(obj), InputType::Object(input_object)) => {
    // Each field lookup gets a FRESH InputField with its OWN is_parameterizable
    // There is NO inheritance from the parent field
    for (name, value) in obj {
        if let Some(field) = input_object.find_field(&name) {
            // field.is_parameterizable() returns THIS field's flag, not inherited
            self.parse_input_field(
                selection_path.clone(),
                argument_path.add(&name),
                field,  // <-- This InputField has its own is_parameterizable
                value,
                query_schema,
            )?;
        }
    }
}
```

### Lists are parameterizable as whole values only:

List-accepting fields (`in`, `notIn`, `hasSome`, `hasEvery`) are parameterizable, but with an important constraint: **the entire list must be a placeholder, not individual elements**.

**Valid:**
- `in: $listParam` - placeholder represents the entire list ✓

**Invalid:**
- `in: [1, 2, $param]` - mixing literals and placeholders ✗
- `in: [$a, $b, $c]` - multiple placeholders for individual elements ✗

**Why?**
- `in: $listParam` uses `ParameterizedRow` for dynamic SQL expansion at execution time
- `in: [1, 2, $param]` would create inconsistent SQL structure (some values inline, some parameterized)
- The list length affects query structure, so all elements must be known together

**Parser behavior:**
```rust
(ArgumentValue::List(values), InputType::List(inner_type)) => {
    // When parsing a literal list, individual elements are NOT parameterizable
    // The whole list would need to be provided as a single placeholder instead
    for value in values {
        self.parse_input_value(
            selection_path.clone(),
            argument_path.clone(),
            value,
            std::slice::from_ref(inner_type.as_ref()),
            query_schema,
            false,  // Elements within a literal list are NOT parameterizable
        )?;
    }
}
```

> **Note:** When a user provides `in: $placeholder`, it arrives as `ArgumentValue::Scalar(PrismaValue::Placeholder(...))`, not as `ArgumentValue::List(...)`. The list case above only applies when the user provides a literal list like `in: [1, 2, 3]`.

---

## Task 4.5: Add Unit Tests

**File:** `query-compiler/core/src/query_document/` (find existing test files)

### Search for Existing Tests

```bash
find query-compiler/core -name "*.rs" -exec grep -l "#\[test\]" {} \;
grep -rn "mod tests" query-compiler/core/src/
```

### Test Cases for Placeholder Validation

```rust
#[cfg(test)]
mod placeholder_validation_tests {
    use super::*;
    use prisma_value::{PrismaValue, Placeholder, PrismaValueType};

    fn create_placeholder(name: &str) -> PrismaValue {
        PrismaValue::Placeholder(Placeholder::new(name, PrismaValueType::Int))
    }

    #[test]
    fn placeholder_in_parameterizable_field_succeeds() {
        // Setup: Create query with placeholder in equals field
        // equals field is marked as parameterizable
        // Assert: Parsing succeeds
    }

    #[test]
    fn placeholder_in_take_fails_with_error() {
        // Setup: Create query with placeholder in take
        // take field is NOT parameterizable
        // Assert: Parsing fails with PlaceholderNotAllowed error
    }

    #[test]
    fn placeholder_in_skip_fails_with_error() {
        // Setup: Create query with placeholder in skip
        // Assert: Parsing fails with PlaceholderNotAllowed error
    }

    #[test]
    fn placeholder_in_nested_filter_succeeds() {
        // Setup: where: { posts: { some: { title: { contains: $search } } } }
        // Assert: Parsing succeeds (contains is parameterizable)
    }

    #[test]
    fn placeholder_in_order_by_fails() {
        // Setup: orderBy: $order
        // Assert: Parsing fails
    }

    #[test]
    fn placeholder_in_create_data_succeeds() {
        // Setup: create: { data: { name: $name } }
        // name field in create data is parameterizable
        // Assert: Parsing succeeds
    }

    #[test]
    fn regular_value_in_non_parameterizable_field_succeeds() {
        // Setup: take: 10 (literal value, not placeholder)
        // Assert: Parsing succeeds (literals always work)
    }

    #[test]
    fn multiple_placeholders_mixed_validity() {
        // Setup: findMany({ 
        //   where: { id: { equals: $id } },  // valid
        //   take: $limit                      // invalid
        // })
        // Assert: Parsing fails on take
    }
}
```

### Test Error Messages

```rust
#[test]
fn placeholder_error_message_is_helpful() {
    // Setup: Create query with placeholder in take
    // Parse and capture error
    // Assert: Error message contains:
    //   - Field name "take"
    //   - Explanation that field doesn't support placeholders
    //   - Suggestion of where placeholders CAN be used
}
```

### GeneratorCall Tests

```rust
#[test]
fn generator_call_in_parameterizable_field_succeeds() {
    // Setup: where: { id: { equals: generatorCall(...) } }
    // Assert: Parsing succeeds
}

#[test]
fn generator_call_in_take_fails() {
    // Setup: take: generatorCall("autoincrement", ...)
    // Assert: Parsing fails with same error type
}
```

### Run Unit Tests

```bash
cargo test -p query-compiler-core placeholder_validation
```

---

## Task 4.6: Integration Tests

**Location:** `query-engine/connector-test-kit-rs/query-engine-tests/`

Integration tests verify end-to-end behavior with actual query execution through the driver adapter infrastructure.

### Check Existing Infrastructure

```bash
ls query-engine/connector-test-kit-rs/query-engine-tests/tests/
```

### Test Scenarios

1. **Valid parameterized query executes**
```
Given: A valid query with placeholders in filter fields
When: The query is compiled and executed
Then: It produces correct results with parameter substitution
```

2. **Invalid parameterized query rejected early**
```
Given: A query with placeholder in take field
When: The query is parsed
Then: A validation error is returned before execution
```

3. **Parameterized filters work with various types**
```
Given: Queries with placeholders for Int, String, DateTime, etc.
When: Compiled and executed
Then: Correct type handling and results
```

### Running Integration Tests

```bash
# Setup test database
make dev-postgres15

# Build QC and driver adapters
make build-driver-adapters-kit-qc

# Run query compiler tests
cargo test -p query-engine-tests -- --nocapture
```

---

## Task 4.7: Regression Verification

Ensure existing functionality is not broken after implementing parser validation.

### Run Full Test Suite

```bash
# Unit tests for all affected crates
cargo test -p schema
cargo test -p dmmf
cargo test -p query-compiler-core

# Full workspace unit tests
make test-unit
```

### Specific Regression Areas

1. **Queries without placeholders** - Must continue to work exactly as before
2. **DMMF output** - Unchanged (field already present from Phase 2)
3. **Error messages** - Existing validation errors unchanged
4. **All connector types** - PostgreSQL, MySQL, SQLite, SQL Server, CockroachDB

---

## Verification Checklist

### Implementation
- [ ] New `placeholder_not_allowed` error type added to validation.rs
- [ ] Error message is clear and helpful
- [ ] `parse_input_value` receives `is_parameterizable` parameter
- [ ] Placeholder validation checks the flag before accepting
- [ ] GeneratorCall validation follows same pattern
- [ ] All call sites updated to pass `is_parameterizable`
- [ ] Object parsing correctly uses each field's own `is_parameterizable` (no inheritance)
- [ ] List parsing correctly applies the list field's `is_parameterizable` to all elements
- [ ] Outdated TODO comment removed from parser.rs

### Unit Tests
- [ ] `cargo build -p query-compiler-core` succeeds
- [ ] Unit tests added for placeholder validation
- [ ] Tests pass: placeholder accepted in filter fields
- [ ] Tests pass: placeholder rejected in take/skip/orderBy
- [ ] Tests pass: GeneratorCall follows same rules
- [ ] Tests pass: error messages are helpful

### Integration Tests
- [ ] Integration tests added for valid parameterized queries
- [ ] Integration tests added for invalid parameterized queries
- [ ] `cargo test -p query-engine-tests` passes

### Regression
- [ ] `cargo test -p schema` passes
- [ ] `cargo test -p dmmf` passes
- [ ] `cargo test -p query-compiler-core` passes
- [ ] `make test-unit` passes
- [ ] Queries without placeholders unaffected

---

## Edge Cases

1. **Placeholder for entire list value**
   - `where: { id: { in: placeholder("ids", IntList) } }`
   - Should succeed - `in` is parameterizable with a whole-list placeholder

2. **Placeholder for individual list elements (invalid)**
   - `where: { id: { in: [1, 2, placeholder("x", Int)] } }`
   - Should fail - individual elements within a literal list cannot be placeholders
   - User must provide either a literal list OR a placeholder for the entire list

3. **Placeholder as object (invalid)**
   - `where: placeholder("filter", Object)`
   - Invalid regardless of parameterizability - placeholders are scalars

4. **Mixed valid/invalid in same query**
   - `findMany({ where: { id: placeholder(...) }, take: placeholder(...) })`
   - Should fail on `take` even though `where.id` is valid

---

## Test Commands Summary

```bash
# Unit tests for parser validation
cargo test -p query-compiler-core placeholder_validation

# All query-compiler-core tests
cargo test -p query-compiler-core

# Integration tests (requires DB setup)
make dev-postgres15
make build-driver-adapters-kit-qc
cargo test -p query-engine-tests

# Full verification
make test-unit
```

---

## Notes

- This validation happens at query parse time, before execution
- The error should be a user-facing error (part of `ValidationError`)
- **Remove the outdated TODO comment** - it refers to the old query engine which no longer exists
- The validation applies to both `Placeholder` and `GeneratorCall` values
- Backward compatibility: queries without placeholders are unaffected
- **No inheritance:** Each `InputField` has its own `is_parameterizable` flag; there's no propagation from parent to child
- **Only scalars matter:** We only check `is_parameterizable` when parsing scalar values; object wrappers can never directly contain placeholders
