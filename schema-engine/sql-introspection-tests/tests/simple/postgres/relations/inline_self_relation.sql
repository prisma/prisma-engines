-- tags=postgres
-- exclude=cockroachdb

CREATE TABLE "Biscuit" (
    id SERIAL PRIMARY KEY,
    next_biscuit_id INTEGER REFERENCES "Biscuit"("id")
);



/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "postgresql"
}

model Biscuit {
  id              Int       @id @default(autoincrement())
  next_biscuit_id Int?
  Biscuit         Biscuit?  @relation("BiscuitToBiscuit", fields: [next_biscuit_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
  other_Biscuit   Biscuit[] @relation("BiscuitToBiscuit")
}
*/
