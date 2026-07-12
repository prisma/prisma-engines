-- tags=mssql

CREATE TABLE [dbo].[A] (
    id INT IDENTITY,
    location GEOGRAPHY,
    CONSTRAINT [A_pkey] PRIMARY KEY (id)
);


/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "sqlserver"
}

model A {
  id       Int                       @id @default(autoincrement())
  location Unsupported("geography")?
}
*/
