-- tags=mssql

-- SQL Server filtered index introspection test

CREATE TABLE [dbo].[User] (
    id INT NOT NULL,
    email NVARCHAR(255) NOT NULL,
    status NVARCHAR(50) NOT NULL,
    CONSTRAINT [User_pkey] PRIMARY KEY (id)
);

CREATE UNIQUE NONCLUSTERED INDEX [email_active_unique] ON [dbo].[User] ([email]) WHERE ([status]='active');

/*
generator js {
  provider        = "prisma-client"
  previewFeatures = ["partialIndexes"]
}

datasource db {
  provider = "sqlserver"
}

model User {
  id     Int    @id
  email  String @unique(map: "email_active_unique", where: raw("([status]='active')")) @db.NVarChar(255)
  status String @db.NVarChar(50)
}
*/
