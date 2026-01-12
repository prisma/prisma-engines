# Quick Reference: Parameterization Implementation

## Key Commands

```bash
# Build
cargo build -p schema -F psl/all
cargo build -p dmmf -F psl/all
cargo build -p query-core -F psl/all

# Test
cargo test -p schema -F psl/all
cargo test -p dmmf -F psl/all
UPDATE_EXPECT=1 cargo test -p dmmf -F psl/all        # Update dmmf snapshots
cargo test -p prisma-fmt -F psl/all
UPDATE_EXPECT=1 cargo test -p prisma-fmt -F psl/all  # Update prisma-fmt snapshots (includes DMMF tests)
cargo test -p query-core -F psl/all
make test-unit                                        # Full unit test suite

# Lint
make pedantic
```

---

## Key Architectural Insight

**Parameterization is a leaf-node property.** Only scalar fields can be parameterizable, never object types.

- **Mark only leaf scalar fields** - `equals`, `contains`, `set`, `increment`, etc.
- **Type reuse handles the rest** - `cursor`, `having`, `where` reuse types that already have parameterizable scalar fields inside
- **No inheritance** - Each `InputField` has its own `is_parameterizable` flag; no propagation from parent to child
- **Parser validation is simple** - Check `is_parameterizable` only when parsing scalar values; object wrappers never directly contain placeholders

---

## Files to Modify by Phase

### Phase 1: Schema Infrastructure
| File | Change |
|------|--------|
| `query-compiler/schema/src/input_types.rs` | Add `is_parameterizable` to `InputField` |

### Phase 2: DMMF Output
| File | Change |
|------|--------|
| `query-compiler/dmmf/src/serialization_ast/schema_ast.rs` | Add field to `DmmfInputField` |
| `query-compiler/dmmf/src/ast_builders/schema_ast_builder/field_renderer.rs` | Emit the flag |
| `prisma-fmt/src/get_dmmf.rs` | Update DMMF snapshots (uses `expect!` macros) |

### Phase 3: Schema Builder
| File | Change |
|------|--------|
| `query-compiler/schema/src/build/input_types/fields/field_filter_types.rs` | Mark filter fields `.parameterizable()` |
| `query-compiler/schema/src/build/input_types/fields/data_input_mapper/create.rs` | Mark create data fields |
| `query-compiler/schema/src/build/input_types/fields/data_input_mapper/update.rs` | Mark update data fields |
| `query-compiler/schema/src/build/input_types/fields/input_fields.rs` | Mark filter input fields |

### Phase 4: Parser Validation
| File | Change |
|------|--------|
| `libs/user-facing-errors/src/query_engine/validation.rs` | Add error type |
| `query-compiler/core/src/query_document/parser.rs` | Validate placeholders |

---

## Parameterizable vs Not

### ✅ Parameterizable (add `.parameterizable()`)
- Filter values: `equals`, `not`, `lt`, `lte`, `gt`, `gte`
- Inclusion filters: `in`, `notIn` (whole-list parameterization)
- String filters: `contains`, `startsWith`, `endsWith`, `search`
- Scalar list filters: `has`, `hasSome`, `hasEvery`
- JSON value filters: `arrayContains`, `arrayStartsWith`, `arrayEndsWith`, `stringContains`, `stringStartsWith`, `stringEndsWith`
- Create/update data fields (scalar values)
- Numeric operations: `set`, `increment`, `decrement`, `multiply`, `divide`
- List operations: `set`, `push`

> **Note:** Only scalar fields can be parameterizable, never object types. Objects like `where`, `cursor`, `having` are wrappers - the scalar fields *inside* them are parameterizable via type reuse.

> **List fields (`in`, `notIn`, `hasSome`, `hasEvery`):** These are parameterized as whole values, not element-by-element. `in: $listParam` is valid, but `in: [1, 2, $param]` is NOT valid.

### ❌ NOT Parameterizable (default, no change needed)

**Structural query arguments (converted to integers or field references):**
- `take`, `skip` (converted to `i64`, used as LIMIT/OFFSET)
- `orderBy` (field references + sort direction enums)
- `distinct` (field references)
- `by` in groupBy (field references)

**Object wrappers (scalar fields inside are auto-parameterizable via type reuse):**
- `cursor` - reuses WhereUniqueInput, scalar fields inside are parameterizable
- `having` - reuses filter types, scalar fields inside are parameterizable
- `where` - reuses filter types, scalar fields inside are parameterizable

**Converted to Rust primitives (would fail with placeholder):**
- `mode` → converted to `QueryMode` enum
- `path` (JSON) → converted to `JsonFilterPath`
- `isEmpty` → converted to `bool`
- `isSet` → converted to `bool`
- `unset` → converted to `bool`

**Relation filter wrappers (take objects, not values):**
- `some`, `every`, `none` (relation list filters)
- `is`, `isNot` (to-one relation filters)

---

## Code Snippets

### InputField with parameterizable flag
```rust
// In input_types.rs
pub struct InputField<'a> {
    // ... existing fields
    is_parameterizable: bool,  // NEW
}

impl InputField<'a> {
    pub fn is_parameterizable(&self) -> bool {
        self.is_parameterizable
    }

    pub(crate) fn parameterizable(mut self) -> Self {
        self.is_parameterizable = true;
        self
    }
}
```

### Marking a filter field
```rust
// Before
simple_input_field(filters::EQUALS, input_type, None).optional()

// After
simple_input_field(filters::EQUALS, input_type, None).optional().parameterizable()
```

### Placeholder validation in parser
```rust
if let ArgumentValue::Scalar(pv @ PrismaValue::Placeholder { .. }) = &value {
    if !is_parameterizable {
        return Err(ValidationError::placeholder_not_allowed(...));
    }
    return Ok(ParsedInputValue::Single(pv.clone()));
}
```

---

## Verification Checklist

- [ ] **Phase 1**: `cargo build -p schema` passes
- [ ] **Phase 2**: `cargo test -p dmmf` passes (after `UPDATE_EXPECT=1`)
- [ ] **Phase 3**: DMMF shows `isParameterizable: true` for filter fields (`equals`, `lt`, `has`, etc.)
- [ ] **Phase 3**: DMMF shows `isParameterizable: false` for `take`/`skip`
- [ ] **Phase 3b**: DMMF shows `isParameterizable: true` for `in`/`notIn`
- [ ] **Phase 3c**: DMMF shows `isParameterizable: true` for `hasSome`/`hasEvery`
- [ ] **Phase 4**: Placeholder in `take` returns validation error
- [ ] **Phase 4**: Placeholder in `equals` parses successfully
- [ ] **Final**: `make test-unit` passes
- [ ] **Final**: `make pedantic` passes

---

## Useful Greps

```bash
# Find all input field creations
grep -rn "input_field\|simple_input_field" query-compiler/schema/src/build/

# Find filter types
grep -rn "EQUALS\|CONTAINS\|IN\b" query-compiler/schema/src/build/

# Find placeholder handling
grep -rn "Placeholder" query-compiler/core/src/

# Find fields converted to bool (NOT parameterizable)
grep -rn "try_into.*bool\|as_boolean" query-compiler/core/src/query_graph_builder/
```

---

## How to Verify Parameterizability

Check `query-compiler/core/src/query_graph_builder/extractors/`:

- **Parameterizable**: Value goes through `as_condition_value()` or `try_into::<PrismaValue>()` → becomes part of `Filter` or `WriteOperation`
- **NOT parameterizable**: Value is converted via `try_into::<bool>()`, `parse_query_mode()`, `parse_json_path()` → Rust primitive
