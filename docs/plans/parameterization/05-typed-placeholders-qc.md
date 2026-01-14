# Task 05: Typed Placeholder Validation in Query Compiler (Follow-up)

## Goal

Support typed placeholders so parameterization can remain safe even when scalar
expectations differ across union contexts.

## Proposed Placeholder Shape

```json
{ "$type": "Param", "value": { "name": "<path>", "type": "<ScalarType>" } }
```

## Scope

- Update the query document parser to accept typed placeholders.
- Validate that the placeholder type matches the input field’s expected scalar
  type (including custom scalars in the future).

## Implementation Notes

- Parser location: `query-compiler/core/src/query_document/parser.rs` around
  the placeholder validation branch (currently lines ~256-265).
- When encountering `PrismaValue::Placeholder`, extract the declared type and
  compare against the expected input type for the current field.
- On mismatch, return a validation error that includes:
  - placeholder name
  - declared type
  - expected type
  The error should be structured so that Client can replace the placeholder name
  with the concrete placeholder value, and the rendered error should be identical
  to the one produced for the literal scalar value without parameterization.

## Dependencies

- Client must emit typed placeholders.

## Files

- `query-compiler/core/src/query_document/parser.rs`
- `libs/user-facing-errors/src/query_engine/validation.rs` (if new error type
  is required)

## Acceptance Criteria

- Typed placeholders are accepted when types match.
- Mismatched types produce a clear validation error.
- Untyped placeholders are not supported anymore.
