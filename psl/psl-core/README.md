# PSL core

This crate is the entry point for the core of the PSL implementation. It
exposes the `Connector` trait and relies on its implementors, but it is itself
decoupled from any specific connector.
