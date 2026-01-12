# Appendix: Code Locations and Snippets

This document provides exact file locations and relevant code snippets for implementing parameterization support.

---

## InputField Definition

**File:** `query-compiler/schema/src/input_types.rs`
**Lines:** ~122-200

```rust
#[derive(Debug, Clone)]
pub struct InputField<'a> {
    pub name: Cow<'a, str>,
    pub default_value: Option<DefaultKind>,

    field_types: Vec<InputType<'a>>,
    is_required: bool,
    requires_other_fields: Vec<Cow<'a, str>>,
    // ADD: is_parameterizable: bool,
}

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
            // ADD: is_parameterizable: false,
        }
    }

    // Existing builder methods: optional(), required(), nullable(), etc.
    // ADD: parameterizable(), parameterizable_if(), is_parameterizable()
}
```

---

## DMMF Schema AST

**File:** `query-compiler/dmmf/src/serialization_ast/schema_ast.rs`
**Lines:** ~63-78

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmmfInputField {
    pub name: String,
    pub is_required: bool,
    pub is_nullable: bool,
    pub input_types: Vec<DmmfTypeReference>,
    // ADD: pub is_parameterizable: bool,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires_other_fields: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<DmmfDeprecation>,
}
```

---

## DMMF Field Renderer

**File:** `query-compiler/dmmf/src/ast_builders/schema_ast_builder/field_renderer.rs`
**Lines:** ~6-24

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
        // ADD: is_parameterizable: input_field.is_parameterizable(),
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

## Query Parser - Placeholder Handling

**File:** `query-compiler/core/src/query_document/parser.rs`
**Lines:** ~250-270

```rust
fn parse_input_value<'a>(
    &self,
    selection_path: SelectionPath<'_>,
    argument_path: ArgumentPath<'_>,
    value: ArgumentValue,
    possible_input_types: &[InputType<'a>],
    query_schema: &'a QuerySchema,
) -> QueryParserResult<ParsedInputValue<'a>> {
    // TODO: make query parsing aware of whether we are using the query compiler,
    // and disallow placeholders and generator calls in the query document if we are not.
    if let ArgumentValue::Scalar(pv @ PrismaValue::Placeholder { .. }) = &value {
        return Ok(ParsedInputValue::Single(pv.clone()));
    }
    if let ArgumentValue::Scalar(pv @ PrismaValue::GeneratorCall { .. }) = &value {
        return Ok(ParsedInputValue::Single(pv.clone()));
    }
    // ... rest of parsing logic
}
```

---

## Schema Builder - Input Field Helper

**File:** `query-compiler/schema/src/build/utils.rs`
**Lines:** ~64-70

```rust
pub(crate) fn input_field<'a>(
    name: impl Into<std::borrow::Cow<'a, str>>,
    field_types: Vec<InputType<'a>>,
    default_value: Option<DefaultKind>,
) -> InputField<'a> {
    InputField::new(name.into(), field_types, default_value, true)
}
```

---

## Pagination Arguments (NOT parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/arguments.rs`
**Lines:** ~154-170

```rust
pub(crate) fn take_argument<'a>(model: &Model) -> InputField<'a> {
    pagination_argument(args::TAKE, model)
    // Should NOT have .parameterizable()
}

pub(crate) fn skip_argument<'a>(model: &Model) -> InputField<'a> {
    pagination_argument(args::SKIP, model)
    // Should NOT have .parameterizable()
}

fn pagination_argument<'a>(arg: &'static str, model: &Model) -> InputField<'a> {
    let arg = input_field(arg, vec![InputType::int()], None).optional();
    if model.has_unique_identifier() {
        arg
    } else {
        arg.with_requires_other_fields([args::ORDER_BY])
    }
}
```

---

## Order By Argument (NOT parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/arguments.rs`
**Lines:** ~143-152

```rust
pub(crate) fn order_by_argument<'a>(
    ctx: &'a QuerySchema,
    container: ParentContainer,
    options: OrderByOptions,
) -> InputField<'_> {
    let order_object_type = InputType::object(order_by_objects::order_by_object_type(ctx, container, options));

    input_field(
        args::ORDER_BY.to_owned(),
        vec![InputType::list(order_object_type.clone()), order_object_type],
        None,
    )
    .optional()
    // Should NOT have .parameterizable()
}
```

---

## Filter Input Fields (SHOULD be parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/input_fields.rs`
**Lines:** ~7-18

```rust
pub(crate) fn filter_input_field(ctx: &'_ QuerySchema, field: ModelField, include_aggregates: bool) -> InputField<'_> {
    let types = field_filter_types::get_field_filter_types(ctx, field.clone(), include_aggregates);
    let nullable = !field.is_required()
        && !field.is_list()
        && match &field {
            ModelField::Scalar(sf) => sf.type_identifier() != TypeIdentifier::Json,
            _ => true,
        };

    input_field(field.name().to_owned(), types, None)
        .optional()
        .nullable_if(nullable)
        // ADD: .parameterizable()
}
```

---

## Filter Types (SHOULD be parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/field_filter_types.rs`

Search for filter operation field creation:

```bash
grep -n "simple_input_field\|input_field" query-compiler/schema/src/build/input_types/fields/field_filter_types.rs
```

Look for fields like:
- `equals`
- `not`
- `in` / `notIn`
- `lt`, `lte`, `gt`, `gte`
- `contains`, `startsWith`, `endsWith`
- `has`, `hasEvery`, `hasSome`

All filter value fields should have `.parameterizable()` added.

---

## Create Data Mapper (SHOULD be parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/data_input_mapper/create.rs`
**Lines:** ~25-34

```rust
impl DataInputFieldMapper for CreateDataInputFieldMapper {
    fn map_scalar<'a>(&self, ctx: &'a QuerySchema, sf: ScalarFieldRef) -> InputField<'a> {
        let typ = map_scalar_input_type_for_field(ctx, &sf);
        let supports_advanced_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);

        match sf.type_identifier() {
            TypeIdentifier::Json if supports_advanced_json => {
                let enum_type = InputType::enum_type(json_null_input_enum(!sf.is_required()));

                input_field(sf.name().to_owned(), vec![enum_type, typ], sf.default_value())
                    .optional_if(!sf.is_required() || sf.default_value().is_some() || sf.is_updated_at())
                    // ADD: .parameterizable()
            }

            _ => input_field(sf.name().to_owned(), vec![typ], sf.default_value())
                .optional_if(!sf.is_required() || sf.default_value().is_some() || sf.is_updated_at())
                .nullable_if(!sf.is_required())
                // ADD: .parameterizable()
        }
    }
}
```

---

## Update Data Mapper (SHOULD be parameterizable)

**File:** `query-compiler/schema/src/build/input_types/fields/data_input_mapper/update.rs`

Similar pattern to create - scalar fields and operation fields (set, increment, etc.) should be parameterizable.

---

## Validation Errors

**File:** `libs/user-facing-errors/src/query_engine/validation.rs`

Search for existing error patterns:

```bash
grep -n "pub fn\|impl.*Error" libs/user-facing-errors/src/query_engine/validation.rs | head -40
```

Add new error for placeholder validation failure.

---

## PrismaValue Placeholder Type

**File:** `libs/prisma-value/src/lib.rs`
**Lines:** ~50-55, ~130-142

```rust
pub enum PrismaValue {
    // ... other variants
    #[serde(serialize_with = "serialize_placeholder")]
    Placeholder(Placeholder),

    #[serde(serialize_with = "serialize_generator_call")]
    GeneratorCall { ... },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Placeholder {
    pub name: Cow<'static, str>,
    pub r#type: PrismaValueType,
}
```

---

## DMMF Tests

**File:** `query-compiler/dmmf/src/tests/tests.rs`

Check existing test patterns:

```bash
head -100 query-compiler/dmmf/src/tests/tests.rs
```

---

## Useful Grep Commands

```bash
# Find all input_field creations
grep -rn "input_field(" query-compiler/schema/src/build/

# Find all simple_input_field creations
grep -rn "simple_input_field(" query-compiler/schema/src/build/

# Find filter-related code
grep -rn "Filter\|filter" query-compiler/schema/src/build/input_types/

# Find pagination-related code
grep -rn "take\|skip\|cursor" query-compiler/schema/src/build/input_types/

# Find constants
cat query-compiler/schema/src/constants.rs
```

---

## Build and Test Commands

```bash
# Build specific crate
cargo build -p schema
cargo build -p dmmf
cargo build -p query-compiler-core

# Test specific crate
cargo test -p schema
cargo test -p dmmf
cargo test -p query-compiler-core

# Update snapshots
UPDATE_EXPECT=1 cargo test -p dmmf

# Full lint check
make pedantic

# Unit tests for workspace
make test-unit
```
