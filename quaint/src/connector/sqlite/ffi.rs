//! Here, we export only the constants we need to avoid pulling in `rusqlite::ffi::*`, in the sibling `error.rs` file,
//! which would break Wasm compilation.
pub const SQLITE_BUSY: i32 = 5;
pub const SQLITE_CONSTRAINT_FOREIGNKEY: i32 = 787;
pub const SQLITE_CONSTRAINT_NOTNULL: i32 = 1299;
pub const SQLITE_CONSTRAINT_PRIMARYKEY: i32 = 1555;
pub const SQLITE_CONSTRAINT_TRIGGER: i32 = 1811;
pub const SQLITE_CONSTRAINT_UNIQUE: i32 = 2067;
