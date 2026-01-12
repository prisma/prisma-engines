# Phase 1: Schema Infrastructure

## Objective

Add `is_parameterizable` field to `InputField` struct and provide builder methods to mark fields as parameterizable.

## Files to Modify

1. `query-compiler/schema/src/input_types.rs`
2. `query-compiler/schema/src/build/utils.rs`

---

## Task 1.1: Add `is_parameterizable` to `InputField`

**File:** `query-compiler/schema/src/input_types.rs`

### Current Code (lines ~122-130)

```rust
#[derive(Debug, Clone)]
pub struct InputField<'a> {
    pub name: Cow<'a, str>,
    pub default_value: Option<DefaultKind>,

    field_types: Vec<InputType<'a>>,
    is_required: bool,
    requires_other_fields: Vec<Cow<'a, str>>,
}
```

### New Code

```rust
#[derive(Debug, Clone)]
pub struct InputField<'a> {
    pub name: Cow<'a, str>,
    pub default_value: Option<DefaultKind>,

    field_types: Vec<InputType<'a>>,
    is_required: bool,
    requires_other_fields: Vec<Cow<'a, str>>,
    is_parameterizable: bool,
}
```

---

## Task 1.2: Update `InputField::new()`

**File:** `query-compiler/schema/src/input_types.rs`

### Current Code (lines ~134-147)

```rust
impl<'a> InputField<'a> {
    pub(crate) fn new(
        name: Cow<'a, str>,
        field_types: Vec<InputType<'a>>,
        default_value: Option<DefaultKind>,
        is_required: bool,
    ) -> InputField<'a> {
        InputField {
            name,
            default_value,
            field_types,
            is_required,
            requires_other_fields: Vec::new(),
        }
    }
```

### New Code

```rust
impl<'a> InputField<'a> {
    pub(crate) fn new(
        name: Cow<'a, str>,
        field_types: Vec<InputType<'a>>,
        default_value: Option<DefaultKind>,
        is_required: bool,
    ) -> InputField<'a> {
        InputField {
            name,
            default_value,
            field_types,
            is_required,
            requires_other_fields: Vec::new(),
            is_parameterizable: false, // Default: not parameterizable
        }
    }
```

---

## Task 1.3: Add getter and builder methods

**File:** `query-compiler/schema/src/input_types.rs`

Add these methods to `impl<'a> InputField<'a>` block (after the existing builder methods like `nullable()`, `optional()`, etc.):

```rust
    /// Returns whether this field accepts placeholder values for parameterized queries.
    pub fn is_parameterizable(&self) -> bool {
        self.is_parameterizable
    }

    /// Marks the field as parameterizable (accepts placeholder values in queries).
    /// 
    /// Parameterizable fields can have their values substituted with placeholders
    /// for query plan caching. This is typically used for filter values and data
    /// fields, but NOT for structural fields like `take`, `skip`, `orderBy`, etc.
    pub(crate) fn parameterizable(mut self) -> Self {
        self.is_parameterizable = true;
        self
    }

    /// Marks the field as parameterizable if the condition is true.
    pub(crate) fn parameterizable_if(self, condition: bool) -> Self {
        if condition { self.parameterizable() } else { self }
    }
```

---

## Task 1.4: Verify Compilation

After making changes, run:

```bash
cargo build -p schema
```

This should compile without errors. All existing code will continue to work since:
- New field has a default value in constructor
- No existing code reads `is_parameterizable` yet

---

## Task 1.5: Add Unit Tests

**Location:** `query-compiler/schema/src/input_types.rs` (add tests module)

Add a test module to verify the `InputField` parameterization behavior:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_field_default_not_parameterizable() {
        let field = InputField::new(
            "test".into(),
            vec![InputType::int()],
            None,
            true,
        );
        assert!(!field.is_parameterizable());
    }

    #[test]
    fn input_field_parameterizable_builder() {
        let field = InputField::new(
            "test".into(),
            vec![InputType::int()],
            None,
            true,
        ).parameterizable();
        assert!(field.is_parameterizable());
    }

    #[test]
    fn input_field_parameterizable_if_true() {
        let field = InputField::new(
            "test".into(),
            vec![InputType::int()],
            None,
            true,
        ).parameterizable_if(true);
        assert!(field.is_parameterizable());
    }

    #[test]
    fn input_field_parameterizable_if_false() {
        let field = InputField::new(
            "test".into(),
            vec![InputType::int()],
            None,
            true,
        ).parameterizable_if(false);
        assert!(!field.is_parameterizable());
    }

    #[test]
    fn input_field_builder_chain_preserves_parameterizable() {
        let field = InputField::new(
            "test".into(),
            vec![InputType::int()],
            None,
            true,
        )
        .parameterizable()
        .optional()
        .nullable();
        
        assert!(field.is_parameterizable());
        assert!(!field.is_required());
    }
}
```

### Run Tests

```bash
cargo test -p schema input_field
```

---

## Verification Checklist

- [ ] `InputField` struct has `is_parameterizable: bool` field
- [ ] `InputField::new()` sets `is_parameterizable: false` by default
- [ ] `is_parameterizable()` getter method exists and is public
- [ ] `parameterizable()` builder method exists (crate-public)
- [ ] `parameterizable_if()` conditional builder exists (crate-public)
- [ ] `cargo build -p schema` succeeds
- [ ] `cargo test -p schema` passes
- [ ] Unit tests verify default, builder, and conditional builder behavior

---

## Notes

- The default is `false` (not parameterizable) because we want explicit opt-in
- The getter is `pub` because it will be accessed by the query parser in a different crate
- The builder methods are `pub(crate)` because only the schema builder should set this
- No changes to `utils.rs` needed in this phase - the `input_field()` helper just calls `InputField::new()` which now has the right default