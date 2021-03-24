use super::*;
use itertools::Itertools;
use std::fmt::Display;

/// A datamodel fragment is the parsed version of a `#<ident>(...)` template string.
#[derive(Debug, PartialEq)]
pub enum DatamodelFragment {
    Id(IdFragment),
}

impl DatamodelFragment {
    pub fn parse(ident: &str, args: Vec<FragmentArgument>) -> TemplatingResult<Self> {
        let fragment = match ident {
            "id" => Self::Id(IdFragment::from_args(args)?),
            ident => return Err(TemplatingError::unknown_ident(ident)),
        };

        Ok(fragment)
    }
}

/// ID field definition, e.g. `#id(id, Int, @id @test.SmallInt)`
#[derive(Debug, PartialEq)]
pub struct IdFragment {
    pub field_name: String,
    pub field_type: String,
    pub directives: Vec<Directive>,
}

impl IdFragment {
    /// Note: This type of parsing can probably be done with nom as well, but may be less readable.
    fn from_args(args: Vec<FragmentArgument>) -> TemplatingResult<Self> {
        if args.len() < 3 {
            return Err(TemplatingError::num_args("id", 3, args.len()));
        }

        let mut args = args.into_iter();
        let (field_name, field_type) = args.next_tuple().unwrap();

        let field_name = field_name.into_value_string()?;
        let field_type = field_type.into_value_string()?;
        let directives = args
            .into_iter()
            .map(|arg| arg.into_directive())
            .collect::<TemplatingResult<Vec<_>>>()?;

        Ok(Self {
            field_name,
            field_type,
            directives,
        })
    }

    // pub fn has_directive(&self, name: &str) -> bool {
    //     self.directives.iter().any(|dir| dir.ident == name)
    // }

    /// Function to update receives a mutable reference to directive with `name`, if it already exists.
    /// The function `f` may choose to return a new directive that will be inserted into the list of directives.
    pub fn upsert_directive<F>(&mut self, name: &str, f: F)
    where
        F: Fn(Option<&mut Directive>) -> Option<Directive>,
    {
        let pos = self.directives.iter().position(|dir| dir.ident == name);
        let existing = pos.and_then(|pos| self.directives.get_mut(pos));

        if let Some(new) = f(existing) {
            self.directives.push(new);
        }
    }
}

impl ToString for IdFragment {
    fn to_string(&self) -> String {
        format!(
            "{} {} {}",
            self.field_name,
            self.field_type,
            self.directives.iter().map(|dir| format!("{}", dir)).join(" ")
        )
    }
}

/// A field directive, e.g. `@map("_id")`.
#[derive(Debug, PartialEq)]
pub struct Directive {
    pub ident: String,
    pub args: Vec<String>,
}

impl Directive {
    pub fn new(ident: &str, args: Vec<&str>) -> Self {
        Self {
            ident: ident.to_owned(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

impl Display for Directive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.args.is_empty() {
            write!(f, r#"@{}"#, self.ident)
        } else {
            write!(f, r#"@{}({})"#, self.ident, self.args.join(", "))
        }
    }
}

/// Wrapper for general argument parsing.
#[derive(Debug, PartialEq)]
pub enum FragmentArgument {
    Value(String),
    Directive(Directive),
}

impl FragmentArgument {
    fn into_value_string(self) -> TemplatingResult<String> {
        match self {
            FragmentArgument::Value(s) => Ok(s),
            FragmentArgument::Directive(_) => Err(TemplatingError::argument_error(
                "unknown",
                format!("Expected Value argument, got: {:?}", self),
            )),
        }
    }

    fn into_directive(self) -> TemplatingResult<Directive> {
        match self {
            FragmentArgument::Value(_) => Err(TemplatingError::argument_error(
                "unknown",
                format!("Expected Directive argument, got: {:?}", self),
            )),
            FragmentArgument::Directive(dir) => Ok(dir),
        }
    }
}
