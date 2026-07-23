-- tags=postgres
-- exclude=cockroachdb

CREATE TABLE "Category" (
    id serial NOT NULL PRIMARY KEY,
    name character varying(255) DEFAULT 'a " b"c d'::character varying NOT NULL
);


/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "postgresql"
}

model Category {
  id   Int    @id @default(autoincrement())
  name String @default("a \" b\"c d") @db.VarChar(255)
}
*/
