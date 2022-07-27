-- tags=mssql

CREATE TABLE a (
    id INT IDENTITY,
    CONSTRAINT a_pkey PRIMARY KEY (id)
);

CREATE TABLE b (
    id INT IDENTITY,
    a_id INT NOT NULL

    CONSTRAINT b_pkey PRIMARY KEY (id),
    CONSTRAINT asdf
        FOREIGN KEY (a_id) REFERENCES a(id)
            ON DELETE CASCADE
            ON UPDATE NO ACTION
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
  id   Int @id @default(autoincrement())
  a_id Int
  a    a   @relation(fields: [a_id], references: [id], onDelete: Cascade, onUpdate: NoAction, map: "asdf")
}
*/
