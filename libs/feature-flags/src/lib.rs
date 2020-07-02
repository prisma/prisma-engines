//! How to implement a feature flag for Prisma:
//! - Add a bool field to the `FeatureFlags` struct.
//! - Add the str equivalent of the flag to the `add_flag` function match.
//! - Add the str equivalent of the flag to the `enable_all()` function.
//!
//! How to use a feature flag:
//! - Make sure that the flags are initialized in the app stack with `feature_flags::initialize(_)`.
//! - Use the flag in crates that have a dependency on the feature flags crate with: `feature_flags::get().<bool_flag_name>`.

use failure::Fail;
use once_cell::sync::OnceCell;

static FEATURE_FLAGS: OnceCell<FeatureFlags> = OnceCell::new();

#[derive(Debug, Fail)]
pub enum FeatureFlagError {
    #[fail(display = "Invalid feature flag: {}", _0)]
    InvalidFlag(String),
}

pub type Result<T> = std::result::Result<T, FeatureFlagError>;

#[derive(Debug, Default)]
pub struct FeatureFlags {
    /// Transactional batches support in the QE.
    pub transaction: bool,

    /// `connectOrCreate` nested query in the QE.
    pub connect_or_create: bool,
}

impl FeatureFlags {
    fn add_flag(&mut self, flag: &str) -> Result<()> {
        match flag {
            "all" => self.enable_all(),
            "transaction" => self.transaction = true,
            "connectOrCreate" => self.connect_or_create = true,
            _ => Err(FeatureFlagError::InvalidFlag(flag.to_owned()))?,
        };

        Ok(())
    }

    fn enable_all(&mut self) {
        self.transaction = true;
        self.connect_or_create = true;
    }
}

/// Initializes the feature flags with given flags.
/// Noop if already initialized.
pub fn initialize(from: &[String]) -> Result<()> {
    FEATURE_FLAGS
        .get_or_try_init(|| {
            from.into_iter().try_fold(FeatureFlags::default(), |mut acc, flag| {
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
