-- tags=postgres
-- exclude=cockroachdb

CREATE TABLE "Biscuit" (
    id SERIAL PRIMARY KEY
);

CREATE TABLE "_BiscuitToBiscuit" (
    "A" INTEGER REFERENCES "Biscuit"("id"),
    "B" INTEGER REFERENCES "Biscuit"("id")
);

CREATE UNIQUE INDEX "AB_unique" ON "_BiscuitToBiscuit"("A","B");
CREATE INDEX "B_index" ON "_BiscuitToBiscuit"("B");


/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

model Biscuit {
  id        Int       @id @default(autoincrement())
  Biscuit_A Biscuit[] @relation("BiscuitToBiscuit")
  Biscuit_B Biscuit[] @relation("BiscuitToBiscuit")
}
*/
