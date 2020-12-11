use crate::{CoreError, CoreResult};
use enumflags2::BitFlags;
use migration_connector::MigrationFeature;

/// A tool to prevent using unfinished features from the Migration Engine.
#[derive(Clone, Copy, Debug)]
pub struct GateKeeper {
    blacklist: BitFlags<MigrationFeature>,
    whitelist: BitFlags<MigrationFeature>,
}

impl GateKeeper {
    /// Creates a new instance, blocking features defined in the constructor.
    pub fn new(whitelist: BitFlags<MigrationFeature>) -> Self {
        Self {
            blacklist: BitFlags::from(MigrationFeature::NativeTypes),
            whitelist,
        }
    }

    /// Returns an error if any of the given features are blocked.
    pub fn any_blocked(&self, features: BitFlags<MigrationFeature>) -> CoreResult<()> {
        if self.whitelist.contains(features) {
            return Ok(());
        }

        let blocked = !self.whitelist & self.blacklist & features;

        if blocked.is_empty() {
            Ok(())
        } else {
            Err(CoreError::GatedPreviewFeatures(
                blocked.iter().map(|feat| format!("{}", feat)).collect(),
            ))
        }
    }
}
