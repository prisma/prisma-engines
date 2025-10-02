-- tags=mysql
-- exclude=vitess

CREATE TABLE A (
    id INT PRIMARY KEY,
    enum_column ENUM('don''t know') DEFAULT 'don''t know' NOT NULL
)
/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "mysql"
  url      = env("DATABASE_URL")
}

model A {
  id          Int           @id
  enum_column A_enum_column @default(don_t_know)
}

enum A_enum_column {
  don_t_know @map("don't know")
}
*/
