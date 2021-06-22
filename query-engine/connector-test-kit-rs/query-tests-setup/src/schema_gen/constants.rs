pub const SIMPLE_ID: &str = "id            String    @id @default(cuid())";
pub const COMPOUND_ID: &str = "id_1          String        @default(cuid())\n
id_2          String        @default(cuid())\n
@@id([id_1, id_2])";
pub const NO_ID: &str = "";

pub const SIMPLE_ID_OPTIONS: [&str; 1] = [SIMPLE_ID];
pub const FULL_ID_OPTIONS: [&str; 3] = [SIMPLE_ID, COMPOUND_ID, NO_ID];
