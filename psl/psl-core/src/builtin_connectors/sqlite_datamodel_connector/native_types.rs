use crate::builtin_connectors::geometry::GeometryParams;

crate::native_type_definition! {
    /// The SQLite native type enum.
    SQLiteType;
    Geometry(GeometryParams) -> Geometry,
}
