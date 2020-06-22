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
    pub transactions: bool,

    /// `connectOrCreate` nested query in the QE.
    pub create_or_connect: bool,
}

impl FeatureFlags {
    fn add_flag(&mut self, flag: &str) -> Result<()> {
        match flag {
            "transaction" => self.transactions = true,
            "createOrConnect" => self.create_or_connect = true,
            _ => Err(FeatureFlagError::InvalidFlag(flag.to_owned()))?,
        };

        Ok(())
    }
}

/// Initializes the feature flags with given flags.
/// Noop if already initialized.
pub fn initialize<'a>(from: impl Iterator<Item = &'a str>) -> Result<()> {
    FEATURE_FLAGS
        .get_or_try_init(|| {
            from.into_iter().try_fold(FeatureFlags::default(), |mut acc, flag| {
                acc.add_flag(flag)?;
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
