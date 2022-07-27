-- tags=mssql

CREATE TABLE [a] (
    [id] INTEGER IDENTITY,
    CONSTRAINT a_pkey PRIMARY KEY CLUSTERED ([id])
);

CREATE TABLE [b] (
    [id] INTEGER IDENTITY,
    [a_id] INTEGER,

    CONSTRAINT b_pkey PRIMARY KEY CLUSTERED ([id]),
    CONSTRAINT asdf
        FOREIGN KEY (a_id) REFERENCES a(id)
            ON DELETE SET NULL
            ON UPDATE CASCADE
);


/*
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "sqlserver"
  url      = "env(TEST_DATABASE_URL)"
}

model a {
  id Int @id @default(autoincrement())
  b  b[]
}

model b {
  id   Int  @id @default(autoincrement())
  a_id Int?
  a    a?   @relation(fields: [a_id], references: [id], map: "asdf")
}
*/
