-- tags=postgres
-- exclude=cockroachdb

CREATE TABLE "Biscuit" (
    id SERIAL PRIMARY KEY,
    next_biscuit_id INTEGER REFERENCES "Biscuit"("id")
);


/*
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = "env(TEST_DATABASE_URL)"
}

model Biscuit {
  id              Int       @id @default(autoincrement())
  next_biscuit_id Int?
  Biscuit         Biscuit?  @relation("BiscuitToBiscuit", fields: [next_biscuit_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
  other_Biscuit   Biscuit[] @relation("BiscuitToBiscuit")
}
*/
