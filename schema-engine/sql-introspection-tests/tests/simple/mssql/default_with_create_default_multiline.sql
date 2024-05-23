-- tags=mssql2017

CREATE TABLE a (
     id INT IDENTITY,
     savings INT,
     CONSTRAINT [A_pkey] PRIMARY KEY (id)
);

EXEC('/* This is a comment */' + 
     'CREATE DEFAULT NEARLY_NOTHING AS 0');
EXEC('sp_bindefault ''NEARLY_NOTHING'', ''a.savings''');
/*
generator js {
  provider = "prisma-client-js"
}

datasource db {
  provider = "sqlserver"
  url      = env("DATABASE_URL")
}

model a {
  id      Int  @id(map: "A_pkey") @default(autoincrement())
  savings Int?
}
*/
