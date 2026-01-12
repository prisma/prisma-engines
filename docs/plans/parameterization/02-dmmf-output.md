# Phase 2: DMMF Output

## Objective

Expose the `isParameterizable` flag in the DMMF (Data Model Meta Format) so that the Prisma Client generator can consume this information to build efficient parameterization data structures.

## Files to Modify

1. `query-compiler/dmmf/src/serialization_ast/schema_ast.rs`
2. `query-compiler/dmmf/src/ast_builders/schema_ast_builder/field_renderer.rs`

---

## Task 2.1: Add `is_parameterizable` to `DmmfInputField`

**File:** `query-compiler/dmmf/src/serialization_ast/schema_ast.rs`

### Current Code (lines ~63-78)

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputField {
    pub name: String,
    pub is_required: bool,
    pub is_nullable: bool,
    pub input_types: Vec<DmmfTypeReference>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires_other_fields: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<DmmfDeprecation>,
}
```

### New Code

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputField {
    pub name: String,
    pub is_required: bool,
    pub is_nullable: bool,
    pub input_types: Vec<DmmfTypeReference>,
    pub is_parameterizable: bool,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires_other_fields: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<DmmfDeprecation>,
}
```

### JSON Output

This will serialize as:

```json
{
  "name": "equals",
  "isRequired": false,
  "isNullable": true,
  "inputTypes": [...],
  "isParameterizable": true
}
```

---

## Task 2.2: Update `render_input_field` to emit the flag

**File:** `query-compiler/dmmf/src/ast_builders/schema_ast_builder/field_renderer.rs`

### Current Code (lines ~6-24)

```rust
pub(super) fn render_input_field<'a>(input_field: &InputField<'a>, ctx: &mut RenderContext<'a>) -> DmmfInputField {
    let type_references = render_input_types(input_field.field_types(), ctx);
    let nullable = input_field
        .field_types()
        .iter()
        .any(|typ| matches!(typ, InputType::Scalar(ScalarType::Null)));

    DmmfInputField {
        name: input_field.name.to_string(),
        input_types: type_references,
        is_required: input_field.is_required(),
        is_nullable: nullable,
        requires_other_fields: input_field
            .requires_other_fields()
            .iter()
            .map(|f| f.to_string())
            .collect(),
        deprecation: None,
    }
}
```

### New Code

```rust
pub(super) fn render_input_field<'a>(input_field: &InputField<'a>, ctx: &mut RenderContext<'a>) -> DmmfInputField {
    let type_references = render_input_types(input_field.field_types(), ctx);
    let nullable = input_field
        .field_types()
        .iter()
        .any(|typ| matches!(typ, InputType::Scalar(ScalarType::Null)));

    DmmfInputField {
        name: input_field.name.to_string(),
        input_types: type_references,
        is_required: input_field.is_required(),
        is_nullable: nullable,
        is_parameterizable: input_field.is_parameterizable(),
        requires_other_fields: input_field
            .requires_other_fields()
            .iter()
            .map(|f| f.to_string())
            .collect(),
        deprecation: None,
    }
}
```

---

## Task 2.3: Verify Compilation

After making changes, run:

```bash
cargo build -p dmmf
```

---

## Task 2.4: Update DMMF Snapshot Tests

The DMMF crate has snapshot tests that will need updating. Additionally, `prisma-fmt` has DMMF-related tests that also need updating.

**Test locations:**
- `query-compiler/dmmf/src/tests/`
- `prisma-fmt/src/get_dmmf.rs`

### Update Existing Snapshots

Run tests and update snapshots for both packages:

```bash
UPDATE_EXPECT=1 cargo test -p dmmf -F psl/all
UPDATE_EXPECT=1 cargo test -p prisma-fmt -F psl/all
```

Review the snapshot diffs to ensure `isParameterizable` appears correctly:
- Should be `false` for all fields initially (until Phase 3 marks fields as parameterizable)
- After Phase 3, filter fields should show `true`

### Verify Snapshot Content

Review the updated snapshots to ensure:

1. **All input fields have `isParameterizable`**:
```json
{
  "name": "equals",
  "isRequired": false,
  "isNullable": true,
  "inputTypes": [...],
  "isParameterizable": false
}
```

2. **Field is present for all input types** - both filter fields and structural fields should have the property

### Add New Test Cases (Optional)

If explicit test cases are desired, add them to `query-compiler/dmmf/src/tests/`:

```rust
#[test]
fn test_input_field_has_is_parameterizable() {
    // Create a schema with a model that has filters
    // Render to DMMF
    // Assert that all input fields have isParameterizable property
}
```

### DMMF Inspection

To manually inspect DMMF output, you can add a helper test or utility in the `dmmf` crate that writes JSON to a file:

```rust
#[test]
fn dump_dmmf_for_inspection() {
    let schema = r#"
        datasource db {
            provider = "postgresql"
            url = "postgres://localhost:5432/test"
        }
        model User {
            id   Int    @id
            name String
        }
    "#;
    let dmmf = dmmf::from_precomputed_parts(/* ... */);
    std::fs::write("dmmf_debug.json", serde_json::to_string_pretty(&dmmf).unwrap()).unwrap();
}
```

Then inspect with:
```bash
jq '.datamodel.inputObjectTypes.prisma[0].fields[0] | keys' dmmf_debug.json
```

Alternatively, the DMMF snapshot tests themselves serve as the canonical verification - review the `expect!` macro contents directly.

---

## Verification Checklist

- [ ] `DmmfInputField` struct has `is_parameterizable: bool` field
- [ ] Field is serialized as `isParameterizable` in JSON (camelCase)
- [ ] `render_input_field` reads from `input_field.is_parameterizable()`
- [ ] `cargo build -p dmmf` succeeds
- [ ] `cargo test -p dmmf` passes (after updating snapshots)
- [ ] `cargo test -p prisma-fmt` passes (after updating snapshots)
- [ ] DMMF JSON output includes `isParameterizable` for all input fields
- [ ] Snapshot diffs reviewed and make sense (all fields show `false` until Phase 3)

---

## Example DMMF Output

After this phase, a filter field in DMMF will look like:

```json
{
  "inputObjectTypes": {
    "prisma": [
      {
        "name": "IntFilter",
        "constraints": { "maxNumFields": null, "minNumFields": null },
        "fields": [
          {
            "name": "equals",
            "isRequired": false,
            "isNullable": true,
            "inputTypes": [
              { "type": "Int", "location": "scalar", "isList": false }
            ],
            "isParameterizable": false
          }
        ]
      }
    ]
  }
}
```

Note: `isParameterizable` will be `false` until Phase 3 marks the appropriate fields.

---

## Notes

- The field is NOT marked with `#[serde(skip_serializing_if = ...)]` because we always want it present for clarity
- This is a non-breaking change to DMMF - clients that don't know about this field will ignore it
- The Prisma Client generator will be updated separately to consume this field