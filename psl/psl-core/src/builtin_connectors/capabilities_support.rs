use crate::datamodel_connector::{Connector, ConnectorCapabilities, ConnectorCapability};
use cfg_if::cfg_if;

cfg_if! {
    // if built only for mysql
    if #[cfg(all(feature="mysql", not(any(feature = "postgresql", feature="sqlite", feature = "cockroachdb", feature="mssql", feature="mongodb"))))] {
        #[inline(always)]
        const fn can_have_capability_impl(capability: ConnectorCapability) -> bool {
            check_comptime_capability(super::mysql_datamodel_connector::CAPABILITIES, capability)
        }

        pub fn has_capability(_: &dyn Connector, capability: ConnectorCapability) -> bool {
            can_have_capability_impl(capability)
        }
    // if built only for sqlite
    } else if #[cfg(all(feature="sqlite", not(any(feature = "postgresql", feature="mysql", feature = "cockroachdb", feature="mssql", feature="mongodb"))))] {
        #[inline(always)]
        const fn can_have_capability_impl(capability: ConnectorCapability) -> bool {
            check_comptime_capability(super::sqlite_datamodel_connector::CAPABILITIES, capability)
        }

        #[inline(always)]
        pub fn has_capability(_: &dyn Connector, capability: ConnectorCapability) -> bool {
            can_have_capability_impl(capability)
        }
    // if built only for postgresql
    } else if #[cfg(all(feature="postgresql", not(any(feature = "sqlite", feature="mysql", feature = "cockroachdb", feature="mssql", feature="mongodb"))))] {
        #[inline(always)]
        const fn can_have_capability_impl(capability: ConnectorCapability) -> bool {
            check_comptime_capability(super::postgres_datamodel_connector::CAPABILITIES, capability)
        }

        #[inline(always)]
        pub fn has_capability(_: &dyn Connector, capability: ConnectorCapability) -> bool {
            can_have_capability_impl(capability)
        }
    // any other build configuration
    } else {
        #[inline(always)]
        const fn can_have_capability_impl(_: ConnectorCapability) -> bool {
            true
        }

        #[inline(always)]
        pub fn has_capability(connector: &dyn Connector, capability: ConnectorCapability) -> bool {
            connector.capabilities().contains(capability)
        }
    }
}

/// Helper function for determining if engine, compiled with the current settings,
/// can potentially have provided capability on. Useful for single-connector builds and can
/// be used to exclude certain code that we know for sure can't be executed for current connector.
/// Has no effect on multi-connector builds
/// # Example
/// ```ignore
/// if !can_have_capability(ConnectorCapability::FullTextSearch) {
///    unreachable!()
/// }
/// ... // if compiled for a single connector, optimizer will exclude the following code if connector does not support full text search
/// ```
#[inline(always)]
pub const fn can_have_capability(cap: ConnectorCapability) -> bool {
    can_have_capability_impl(cap)
}

/// Marks the code as reachable only by the connectors,
/// having the specific capability.
/// Optimizer usually can optimize the code away if none of the connectors
/// current build supports the capability.
///
/// If we are within a single connector build that has no such capability,
/// and the code marked with this macro is reached, it will panic.
#[macro_export]
macro_rules! reachable_only_with_capability {
    ($cap: expr) => {
        if !$crate::builtin_connectors::can_have_capability($cap) {
            core::unreachable!()
        }
    };
}

#[inline(always)]
#[allow(dead_code)] // not used if more than one connector is built
const fn check_comptime_capability(capabilities: ConnectorCapabilities, cap: ConnectorCapability) -> bool {
    (capabilities.bits_c() & (cap as u64)) > 0
}

#[inline(always)]
pub const fn can_support_relation_load_strategy() -> bool {
    can_have_capability(ConnectorCapability::LateralJoin)
        || can_have_capability(ConnectorCapability::CorrelatedSubqueries)
}
