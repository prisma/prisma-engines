use super::*;
use query_tests_setup::ConnectorTag;

pub fn validate_suite(suite: &Option<String>, on_module: bool) -> Result<(), darling::Error> {
    if suite.is_none() && !on_module {
        return Err(darling::Error::custom(
          "A test suite name annotation on either the test mod (#[test_suite]) or the test (suite = \"name\") is required.",
      ));
    }

    Ok(())
}

pub fn connectors_to_test(only: &OnlyConnectorTags, exclude: &ExcludeConnectorTags) -> Vec<ConnectorTag> {
    if !only.is_empty() {
        if !exclude.is_empty() {
            return only
                .tags()
                .to_vec()
                .into_iter()
                .filter(|tag| !exclude.tags().contains(tag))
                .collect();
        }
        only.tags().to_vec()
    } else if !exclude.is_empty() {
        let all = ConnectorTag::all();
        let exclude = exclude.tags();

        all.into_iter().filter(|tag| !exclude.contains(tag)).collect()
    } else {
        ConnectorTag::all()
    }
}
