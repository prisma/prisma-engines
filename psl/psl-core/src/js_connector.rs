#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ConnectorMode {
    /// Rust drivers are used in Query Engine.
    Rust,

    /// JS drivers are used in Query Engine.
    Js,
}
