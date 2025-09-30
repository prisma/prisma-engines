-- tags=mssql

CREATE TABLE a (
    id INT IDENTITY,
    CONSTRAINT a_pkey PRIMARY KEY (id)
);

CREATE TABLE b (
    id INT IDENTITY,
    a_id INT NOT NULL,

    CONSTRAINT asdf FOREIGN KEY (a_id) REFERENCES a(id)
        ON DELETE NO ACTION
        ON UPDATE CASCADE,
    CONSTRAINT b_pkey PRIMARY KEY (id)
);


/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "sqlserver"
  url      = env("DATABASE_URL")
}

model a {
  id Int @id @default(autoincrement())
  b  b[]
}

model b {
  id   Int @id @default(autoincrement())
  a_id Int
  a    a   @relation(fields: [a_id], references: [id], map: "asdf")
}
*/
