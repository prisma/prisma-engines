# Contributing to datamodel

## Testing

Run `cargo test` in the `core` crate.

## Style guidelines

- Avoid unnecessary object-like structs. Use free-standing functions and context structs.
- Function arguments should generally be ordered from more specific to less specific. Any context-like arguments should come last. Mutable arguments also should tend to come last, since they're for generally for writing (side-effects, context) rather than reading.
