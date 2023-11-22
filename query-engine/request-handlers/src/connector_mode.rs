#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ConnectorMode {
    /// Indicates that Rust drivers are used in Query Engine.
    #[cfg(feature = "native")]
    Rust,

    /// Indicates that JS drivers are used in Query Engine.
    Js,
}
