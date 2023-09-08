/// Macro that replaces JSON nullability strings with the
/// appropriate representation based on the connector capabilities.
/// Allows us to reuse tests instead of copying.
#[macro_export]
macro_rules! jNull {
    ($capabilities:expr, $s:expr) => {
        if !$capabilities.contains(ConnectorCapability::AdvancedJsonNullability) {
            $s.replace("DbNull", "null")
                .replace("JsonNull", "\"null\"")
                .replace("AnyNull", "null")
        } else {
            $s.to_owned()
        }
    };
}
