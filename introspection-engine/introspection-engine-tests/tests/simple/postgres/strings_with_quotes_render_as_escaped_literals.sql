-- tags=postgres
-- exclude=cockroachdb

CREATE TABLE "Category" (
    id serial NOT NULL PRIMARY KEY,
    name character varying(255) DEFAULT 'a " b"c d'::character varying NOT NULL
);

/*
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = "env(TEST_DATABASE_URL)"
}

model Category {
  id   Int    @id @default(autoincrement())
  name String @default("a \" b\"c d") @db.VarChar(255)
}
*/
