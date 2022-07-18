-- tags=mssql

CREATE TABLE [dbo].[A] (
    id INT IDENTITY,
    location GEOGRAPHY,
    CONSTRAINT [A_pkey] PRIMARY KEY (id)
);

/*
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "sqlserver"
  url      = "env(TEST_DATABASE_URL)"
}

model A {
  id       Int                       @id @default(autoincrement())
  location Unsupported("geography")?
}
*/
