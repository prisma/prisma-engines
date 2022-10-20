use std::fmt;

use crate::Enum;

/// The PSL data model declaration.
#[derive(Default, Debug)]
pub struct Datamodel<'a> {
    enums: Vec<Enum<'a>>,
}

impl<'a> Datamodel<'a> {
    /// Create a new empty data model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an enum block to the data model.
    ///
    /// ```ignore
    /// enum Foo { // <
    ///   Bar      // < this
    /// }          // <
    /// ```
    pub fn push_enum(&mut self, r#enum: Enum<'a>) {
        self.enums.push(r#enum);
    }
}

impl<'a> fmt::Display for Datamodel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r#enum in self.enums.iter() {
            writeln!(f, "{enum}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use expect_test::expect;

    #[test]
    fn simple_data_model() {
        let mut traffic_light = Enum::new("TrafficLight");

        traffic_light.push_variant("Red");
        traffic_light.push_variant("Yellow");
        traffic_light.push_variant("Green");

        let mut cat = Enum::new("Cat");
        cat.push_variant("Asleep");
        cat.push_variant("Hungry");

        let mut data_model = Datamodel::new();
        data_model.push_enum(traffic_light);
        data_model.push_enum(cat);

        let expected = expect![[r#"
            enum TrafficLight {
              Red
              Yellow
              Green
            }

            enum Cat {
              Asleep
              Hungry
            }
        "#]];

        let rendered = psl::reformat(&format!("{data_model}"), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
