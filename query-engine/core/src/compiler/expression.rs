use query_structure::PrismaValue;

#[derive(Debug)]
pub struct Binding {
    pub name: String,
    pub expr: Expression,
}

impl std::fmt::Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.name, self.expr)
    }
}

#[derive(Debug)]
pub struct DbQuery {
    pub query: String,
    pub params: Vec<PrismaValue>,
}

#[derive(Debug)]
pub enum Expression {
    Seq(Vec<Expression>),
    Get {
        name: String,
    },
    Let {
        bindings: Vec<Binding>,
        expr: Box<Expression>,
    },
    GetFirstNonEmpty {
        names: Vec<String>,
    },
    ReadQuery(DbQuery),
    WriteQuery(DbQuery),
}

impl Expression {
    fn display(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        let indent = "  ".repeat(level);

        match self {
            Self::Seq(exprs) => {
                writeln!(f, "{indent}{{")?;
                for expr in exprs {
                    expr.display(f, level + 1)?;
                    writeln!(f, ";")?;
                }
                write!(f, "{indent}}}")?;
            }
            Self::Get { name } => {
                write!(f, "{indent}get {name}")?;
            }
            Self::Let { bindings, expr } => {
                writeln!(f, "{indent}let")?;
                for Binding { name, expr } in bindings {
                    writeln!(f, "{indent}  {name} =")?;
                    expr.display(f, level + 2)?;
                    writeln!(f, ";")?;
                }
                writeln!(f, "{indent}in")?;
                expr.display(f, level + 1)?;
            }
            Self::GetFirstNonEmpty { names } => {
                write!(f, "{indent}getFirstNonEmpty")?;
                for name in names {
                    write!(f, " {}", name)?;
                }
            }
            Self::ReadQuery(query) => self.display_query("readQuery", query, f, level)?,
            Self::WriteQuery(query) => self.display_query("writeQuery", query, f, level)?,
        }
        Ok(())
    }

    fn display_query(
        &self,
        op: &str,
        db_query: &DbQuery,
        f: &mut std::fmt::Formatter<'_>,
        level: usize,
    ) -> std::fmt::Result {
        let indent = "  ".repeat(level);
        let DbQuery { query, params } = db_query;
        write!(f, "{indent}{op} {{\n{indent}  {query}\n{indent}}} with {params:?}")
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display(f, 0)
    }
}
