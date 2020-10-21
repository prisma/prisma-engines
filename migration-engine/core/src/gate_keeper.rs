use crate::{CoreError, CoreResult};

/// A tool to prevent using unfinished features from the Migration Engine.
pub struct GateKeeper {
    blacklist: &'static [&'static str],
    whitelist: Vec<String>,
}

impl GateKeeper {
    /// Creates a new instance, blocking features defined in the constructor.
    pub fn new(whitelist: Vec<String>) -> Self {
        Self {
            blacklist: &["nativeTypes", "microsoftSqlServer"],
            whitelist,
        }
    }

    /// Returns an error if any of the given features are blocked.
    pub fn any_blocked<'a, I>(&'a self, features: I) -> CoreResult<()>
    where
        I: Iterator<Item = &'a str>,
    {
        if self.whitelist.iter().any(|s| s == "all") {
            return Ok(());
        }

        let blacklist = self.blacklist;
        let whitelist = &self.whitelist;

        let mut blocked = features
            .filter(move |s| !whitelist.iter().any(|w| s == w) && blacklist.contains(s))
            .peekable();

        if blocked.peek().is_some() {
            Err(CoreError::GatedPreviewFeatures(
                blocked.map(ToString::to_string).collect(),
            ))
        } else {
            Ok(())
        }
    }

    /// Returns a whitelist vector allowing all gated features.
    pub fn allow_all_whitelist() -> Vec<String> {
        vec![String::from("all")]
    }
}
