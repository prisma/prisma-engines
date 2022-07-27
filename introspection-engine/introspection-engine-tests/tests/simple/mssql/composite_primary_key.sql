-- tags=mssql

CREATE TABLE [dbo].[User] (
    [id] INT NOT NULL,
    [name] VARCHAR(255) NOT NULL,
    CONSTRAINT [PK_User] PRIMARY KEY ([id], [name])
)

/*
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "sqlserver"
  url      = "env(TEST_DATABASE_URL)"
}

model User {
  id   Int
  name String @db.VarChar(255)

  @@id([id, name], map: "PK_User")
}
*/
