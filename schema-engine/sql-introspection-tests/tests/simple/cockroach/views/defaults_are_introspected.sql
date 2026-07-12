-- preview_features=views
-- tags=cockroachdb

CREATE TABLE "A" (id SERIAL PRIMARY KEY, val INT DEFAULT 3);
CREATE VIEW "B" AS SELECT id, val FROM "A";



/*
generator js {
  provider        = "prisma-client"
  previewFeatures = ["views"]
}

datasource db {
  provider = "cockroachdb"
}

model A {
  id  BigInt  @id @default(autoincrement())
  val BigInt? @default(3)
}

view B {
  id  BigInt?
  val BigInt?
}
*/
