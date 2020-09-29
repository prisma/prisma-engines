#![allow(non_snake_case)]
//! How to implement a feature flag for Prisma:
//! - Add the desired identifier to the `flags!` macro invocation
//!
//! Note: the stringified version of that ident will be used as the string flag name.
//!
//! How to use a feature flag:
//! - Make sure that the flags are initialized in the app stack with `feature_flags::initialize(_)`.
//! - Use the flag in crates that have a dependency on the feature flags crate with: `feature_flags::get().<bool_flag_name>`.

use once_cell::sync::OnceCell;
use thiserror::Error;

static FEATURE_FLAGS: OnceCell<FeatureFlags> = OnceCell::new();

#[derive(Debug, Error)]
pub enum FeatureFlagError {
    #[error("Invalid feature flag: {0}")]
    InvalidFlag(String),
}

pub type Result<T> = std::result::Result<T, FeatureFlagError>;

macro_rules! flags {
    ($( $field:ident ),*) => {
        #[derive(Debug, Default)]
        pub struct FeatureFlags {
            $(
                pub $field: bool,
            )*
        }

        impl FeatureFlags {
            fn add_flag(&mut self, flag: &str) -> Result<()> {
                match flag {
                    "all" => self.enable_all(),
                    $(
                        stringify!($field) => self.$field = true,
                    )*
                    _ => return Err(FeatureFlagError::InvalidFlag(flag.to_owned())),
                };

                Ok(())
            }

            fn enable_all(&mut self) {
                $(
                    self.$field = true;
                )*
            }
        }
    };
}

// `transaction`: Transactional batches support in the QE.
// `connectOrCreate`: `connectOrCreate` nested query in the QE.
// `atomicNumberOperations`: New and expanded number operations for updates.
// `microsoftSqlServer`: Support for Microsoft SQL Server databases
flags!(transaction, connectOrCreate, atomicNumberOperations, microsoftSqlServer);

/// Initializes the feature flags with given flags.
/// Noop if already initialized.
pub fn initialize(from: &[String]) -> Result<()> {
    FEATURE_FLAGS
        .get_or_try_init(|| {
            from.iter().try_fold(FeatureFlags::default(), |mut acc, flag| {
                acc.add_flag(&flag)?;
                Ok(acc)
            })
        })
        .map(|_| ())
}

/// Returns a reference to the global feature flags.
/// Panics if feature flags are uninitialized.
pub fn get() -> &'static FeatureFlags {
    FEATURE_FLAGS
        .get()
        .expect("Expected feature flags to be initialized before calling get.")
}
