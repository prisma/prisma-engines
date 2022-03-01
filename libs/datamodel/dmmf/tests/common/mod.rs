use datamodel::{Configuration, Datamodel};

pub(crate) use expect_test::expect;

pub(crate) fn parse(datamodel_string: &str) -> Datamodel {
    match datamodel::parse_datamodel(datamodel_string) {
        Ok(s) => s.subject,
        Err(errs) => {
            panic!(
                "Datamodel parsing failed\n\n{}",
                errs.to_pretty_string("", datamodel_string)
            )
        }
    }
}

pub(crate) fn parse_configuration(datamodel_string: &str) -> Configuration {
    match datamodel::parse_configuration(datamodel_string) {
        Ok(c) => c.subject,
        Err(errs) => {
            panic!(
                "Configuration parsing failed\n\n{}",
                errs.to_pretty_string("", datamodel_string)
            )
        }
    }
}
