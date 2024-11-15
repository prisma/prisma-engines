//! Convenient access to a ID generator constants, used by Prisma in psl, Query Engine and Schema Engine.

/// Version of the `uuid()` ID generator supported by Prisma.
pub const UUID_SUPPORTED_VERSIONS: [u8; 2] = [4, 7];

/// Version of the `cuid()` ID generator supported by Prisma.
pub const CUID_SUPPORTED_VERSIONS: [u8; 2] = [1, 2];

/// Default version of the `uuid()` ID generator.
pub const DEFAULT_UUID_VERSION: u8 = 4;

/// Default version of the `cuid()` ID generator.
pub const DEFAULT_CUID_VERSION: u8 = 2;
