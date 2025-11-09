-- tags=mssql2017

CREATE TABLE a (
     id INT IDENTITY,
     savings INT,
     CONSTRAINT [A_pkey] PRIMARY KEY (id)
);

EXEC('CREATE DEFAULT NEARLY_NOTHING AS 0');
EXEC('sp_bindefault ''NEARLY_NOTHING'', ''a.savings''');
/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "sqlserver"
}

model a {
  id      Int  @id(map: "A_pkey") @default(autoincrement())
  savings Int?
}
*/
