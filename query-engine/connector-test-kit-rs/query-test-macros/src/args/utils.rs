pub(crate) fn validate_suite(suite: &Option<String>, on_module: bool) -> Result<(), darling::Error> {
    if suite.is_none() && !on_module {
        return Err(darling::Error::custom(
          "A test suite name annotation on either the test mod (#[test_suite]) or the test (suite = \"name\") is required.",
      ));
    }

    Ok(())
}
