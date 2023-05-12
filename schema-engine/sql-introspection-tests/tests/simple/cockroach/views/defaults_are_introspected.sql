-- preview_features=views
-- tags=cockroachdb

CREATE TABLE "A" (id SERIAL PRIMARY KEY, val INT DEFAULT 3);
CREATE VIEW "B" AS SELECT id, val FROM "A";


/*
generator js {
  provider        = "prisma-client-js"
  previewFeatures = ["views"]
}

datasource db {
  provider = "cockroachdb"
  url      = env("DATABASE_URL")
}

model A {
  id  BigInt  @id @default(autoincrement())
  val BigInt? @default(3)
}

/// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
view B {
  id  BigInt?
  val BigInt?

  @@ignore
}
*/
