# Phase 3a: Enable `orderBy._relevance.search` Parameterization

> **Note:** This is a follow-up task to Phase 3. It should be implemented after the main parameterization work is complete.

## Objective

Enable the `search` field in `orderBy: { _relevance: { search: "..." } }` to accept placeholder values for parameterized queries. Currently, the search string is converted to a Rust `String` early in the extraction process, losing the ability to keep it as a `PrismaValue::Placeholder`.

## Current Code Path

### 1. Extraction (query-compiler/core)

**File:** `query-compiler/core/src/query_graph_builder/extractors/query_arguments.rs`
**Lines:** 175-205

```rust
fn extract_order_by_relevance(
    container: &ParentContainer,
    object: ParsedInputMap<'_>,
    path: Vec<OrderByHop>,
) -> QueryGraphBuilderResult<Option<OrderBy>> {
    let (sort_order, _) = extract_order_by_args(object.get(ordering::SORT).unwrap().clone())?;
    let search: PrismaValue = object.get(ordering::SEARCH).unwrap().clone().try_into()?;
    let search = search.into_string().unwrap();  // <-- PROBLEM: Converts to String, loses Placeholder
    // ...
    Ok(Some(OrderBy::relevance(fields, search, sort_order, path)))
}
```

### 2. Storage (query-structure)

**File:** `query-compiler/query-structure/src/order_by.rs`
**Lines:** 206-211

```rust
pub struct OrderByRelevance {
    pub fields: Vec<ScalarFieldRef>,
    pub sort_order: SortOrder,
    pub search: String,  // <-- Currently String, needs to be PrismaValue
    pub path: Vec<OrderByHop>,
}
```

### 3. SQL Building (sql-query-builder)

**File:** `query-compiler/query-builders/sql-query-builder/src/ordering.rs`
**Lines:** 241-245

```rust
pub(crate) fn compute_joins_relevance(
    &mut self,
    order_by: &OrderByRelevance,
    ctx: &Context<'_>,
) -> (Vec<AliasedJoin>, Expression<'static>) {
    // ...
    let text_search_expr = text_search_relevance(&order_by_columns, order_by.search.clone());
    // order_by.search is String, passed to text_search_relevance which expects impl Into<Cow<'a, str>>
    (joins, text_search_expr.into())
}
```

### 4. Quaint Function

**File:** `quaint/src/ast/function/search.rs`
**Lines:** 40-43, 63-73

```rust
pub struct TextSearchRelevance<'a> {
    pub(crate) exprs: Vec<Expression<'a>>,
    pub(crate) query: Cow<'a, str>,  // <-- Currently Cow<str>, would need to support Value or Expression
}

pub fn text_search_relevance<'a, E, Q>(exprs: &[E], query: Q) -> super::Function<'a>
where
    E: Clone + Into<Expression<'a>>,
    Q: Into<Cow<'a, str>>,  // <-- Constraint prevents passing PrismaValue
{
    // ...
}
```

### 5. SQL Visitor (Postgres example)

**File:** `quaint/src/visitor/postgres.rs`
**Lines:** 642-663

```rust
fn visit_text_search_relevance(&mut self, text_search_relevance: TextSearchRelevance<'a>) -> visitor::Result {
    // ...
    self.surround_with("to_tsquery(", ")", |s| s.visit_parameterized(Value::text(query)))?;
    // query is Cow<str>, converted to Value::text() for parameterization
    // ...
}
```

**File:** `quaint/src/visitor/mysql.rs`
**Lines:** 573-581

```rust
fn visit_matches(&mut self, left: Expression<'a>, right: std::borrow::Cow<'a, str>, not: bool) -> visitor::Result {
    // ...
    self.surround_with("AGAINST (", " IN BOOLEAN MODE)", |s| {
        s.visit_parameterized(Value::text(right))  // Also converts to Value::text()
    })?;
    // ...
}
```

## Analysis

The SQL visitors already parameterize the search string via `Value::text(query)`. The issue is that the `PrismaValue` is converted to `String` too early in the pipeline (step 1), before we have a chance to preserve placeholder information.

## Required Changes

### Option A: Change `search` type to `PrismaValue` (Recommended)

This is the cleanest approach - keep the value as `PrismaValue` throughout and only convert to string at the SQL visitor level.

#### Changes Required:

1. **query-structure/src/order_by.rs**
   ```rust
   pub struct OrderByRelevance {
       pub fields: Vec<ScalarFieldRef>,
       pub sort_order: SortOrder,
       pub search: PrismaValue,  // Changed from String
       pub path: Vec<OrderByHop>,
   }
   ```

2. **query-compiler/core/.../query_arguments.rs**
   ```rust
   fn extract_order_by_relevance(...) -> ... {
       // ...
       let search: PrismaValue = object.get(ordering::SEARCH).unwrap().clone().try_into()?;
       // Remove: let search = search.into_string().unwrap();
       Ok(Some(OrderBy::relevance(fields, search, sort_order, path)))
   }
   ```

3. **sql-query-builder/src/ordering.rs**
   - Change `compute_joins_relevance` to convert `PrismaValue` to the appropriate quaint type
   - Need to handle both `PrismaValue::String(s)` and `PrismaValue::Placeholder(...)` cases

4. **quaint/src/ast/function/search.rs**
   - Option A1: Change `TextSearchRelevance.query` to `Expression<'a>` instead of `Cow<'a, str>`
   - Option A2: Keep as `Cow<'a, str>` but handle placeholder separately in sql-query-builder

5. **quaint/src/visitor/{postgres,mysql}.rs**
   - If using Option A1: Update visitors to handle Expression instead of Cow<str>
   - If using Option A2: No changes needed in quaint

### Option B: Add Expression wrapper in sql-query-builder

Keep `OrderByRelevance.search` as `PrismaValue` but convert it to a quaint `Expression` in the SQL builder, allowing the existing visitor logic to handle parameterization.

This avoids changes to quaint but requires changes in how `text_search_relevance` is called.

## Recommended Approach: Option A2

1. Change `OrderByRelevance.search` from `String` to `PrismaValue`
2. In `sql-query-builder/src/ordering.rs`, convert the `PrismaValue` to string when calling `text_search_relevance`:
   - For `PrismaValue::String(s)`: pass `s` directly (current behavior)
   - For `PrismaValue::Placeholder(...)`: Need special handling - see below

### Challenge: Placeholders in quaint

The quaint `text_search_relevance` function expects a string-like value. For placeholders, we need to either:

1. **Extend quaint** to accept `Value<'a>` or `Expression<'a>` for the query parameter
2. **Handle at a higher level** by not using `text_search_relevance` for placeholders

Given that the visitors already call `Value::text(query)`, extending quaint to accept `Value<'a>` directly would be cleanest:

```rust
// quaint/src/ast/function/search.rs
pub struct TextSearchRelevance<'a> {
    pub(crate) exprs: Vec<Expression<'a>>,
    pub(crate) query: Expression<'a>,  // Changed from Cow<'a, str>
}

pub fn text_search_relevance<'a, E>(exprs: &[E], query: impl Into<Expression<'a>>) -> super::Function<'a>
where
    E: Clone + Into<Expression<'a>>,
{
    let exprs: Vec<Expression> = exprs.iter().map(|c| c.clone().into()).collect();
    let fun = TextSearchRelevance {
        exprs,
        query: query.into(),
    };
    fun.into()
}
```

Then visitors would use `self.visit_expression(query)` instead of `self.visit_parameterized(Value::text(query))`.

## Implementation Tasks

### Task 3a.1: Update `OrderByRelevance` struct

**File:** `query-compiler/query-structure/src/order_by.rs`

- [ ] Change `search: String` to `search: PrismaValue`
- [ ] Update `OrderBy::relevance()` constructor to accept `PrismaValue`

### Task 3a.2: Update extraction to preserve `PrismaValue`

**File:** `query-compiler/core/src/query_graph_builder/extractors/query_arguments.rs`

- [ ] Remove `.into_string().unwrap()` call in `extract_order_by_relevance`
- [ ] Keep `search` as `PrismaValue`

### Task 3a.3: Update quaint `TextSearchRelevance`

**File:** `quaint/src/ast/function/search.rs`

- [ ] Change `query: Cow<'a, str>` to `query: Expression<'a>`
- [ ] Update `text_search_relevance` function signature
- [ ] Add `impl From<&str>` and `impl From<String>` for convenience

### Task 3a.4: Update quaint visitors

**Files:**
- `quaint/src/visitor/postgres.rs`
- `quaint/src/visitor/mysql.rs`
- `quaint/src/visitor/mssql.rs` (if applicable)
- `quaint/src/visitor/sqlite.rs` (if applicable)

- [ ] Change `visit_text_search_relevance` to use `self.visit_expression(query)`
- [ ] Or wrap in `Value::text()` only for string expressions
- [ ] Ensure placeholder values are properly converted to SQL parameters

### Task 3a.5: Update sql-query-builder

**File:** `query-compiler/query-builders/sql-query-builder/src/ordering.rs`

- [ ] Update `compute_joins_relevance` to convert `PrismaValue` to quaint `Expression`
- [ ] Handle `PrismaValue::Placeholder` → appropriate quaint expression
- [ ] Handle `PrismaValue::String` → `Value::text(s).into()`

### Task 3a.6: Mark schema field as parameterizable

The `search` field is created as part of the `_relevance` input object for orderBy.

**Steps to find the field:**

```bash
# Search for relevance-related order by schema building
grep -rn "relevance\|RELEVANCE" query-compiler/schema/src/build/
```

**Files to check:**
- `query-compiler/schema/src/build/input_types/objects/order_by_objects.rs` (likely location)
- Look for where `OrderByRelevanceInput` or similar is defined

**Changes:**
- [ ] Find the `search` field definition in the orderBy relevance input object
- [ ] Add `.parameterizable()` to the `search` field
- [ ] Verify `fields` and `sort` fields do NOT have `.parameterizable()` (they are structural)

**Example expected change:**
```rust
// Before:
input_field("search", vec![InputType::string()], None)

// After:
input_field("search", vec![InputType::string()], None).parameterizable()
```

## Testing

- [ ] Unit test: `OrderByRelevance` with `PrismaValue::String`
- [ ] Unit test: `OrderByRelevance` with `PrismaValue::Placeholder`
- [ ] Integration test: Full-text search with parameterized query
- [ ] DMMF snapshot: Verify `isParameterizable: true` for `search` field

## Dependencies

- Phase 1 (Schema Infrastructure) must be completed first
- Phase 2 (DMMF Output) must be completed first
- Phase 3 (Schema Builder) must be completed first - this task extends the parameterization to cover the `search` field

## Estimated Complexity

- **quaint changes:** Medium - modifying core AST types
- **query-structure changes:** Low - simple type change
- **sql-query-builder changes:** Medium - need to handle PrismaValue → Expression conversion
- **schema changes:** Low - adding `.parameterizable()` call

## Notes

- This is an optional enhancement - the core parameterization feature works without this
- Full-text search is only supported on PostgreSQL and MySQL (not SQLite, MSSQL)
- The `fields` parameter in `_relevance` is structural (field references) and should NOT be parameterizable
- The `sort` parameter is an enum and should NOT be parameterizable