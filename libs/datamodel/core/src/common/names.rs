pub trait NameNormalizer {
    fn camel_case(&self) -> String;

    fn pascal_case(&self) -> String;
}

impl NameNormalizer for String {
    fn camel_case(&self) -> String {
        let mut c = self.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        }
    }

    fn pascal_case(&self) -> String {
        let mut c = self.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }
}

pub struct DefaultNames {}

impl DefaultNames {
    pub fn name_for_unambiguous_relation(from: &str, to: &str) -> String {
        if from < to {
            format!("{}To{}", from, to)
        } else {
            format!("{}To{}", to, from)
        }
    }

    pub fn name_for_ambiguous_relation(from: &str, to: &str, scalar_field: &str) -> String {
        if from < to {
            format!("{}_{}To{}", from, scalar_field, to)
        } else {
            format!("{}To{}_{}", to, from, scalar_field)
        }
    }
}
