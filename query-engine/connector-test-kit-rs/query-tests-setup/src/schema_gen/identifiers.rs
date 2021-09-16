#[derive(Debug, Clone, PartialEq)]
pub enum Identifier {
    Simple,
    Compound,
    None,
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Identifier::Simple => "#id(id, String, @id, @default(cuid()))",
            Identifier::Compound => {
                "id_1          String        @default(cuid())\n
                 id_2          String        @default(cuid())\n
                 @@id([id_1, id_2])"
            }
            Identifier::None => "",
        };

        write!(f, "{}", name)
    }
}
