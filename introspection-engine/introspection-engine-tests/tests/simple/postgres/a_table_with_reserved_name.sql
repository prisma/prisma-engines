-- tags=postgres
-- exclude=cockroachdb

CREATE TABLE "PrismaClient" (
    id SERIAL PRIMARY KEY
);
/*
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = "env(TEST_DATABASE_URL)"
}

/// This model has been renamed to 'RenamedPrismaClient' during introspection, because the original name 'PrismaClient' is reserved.
model RenamedPrismaClient {
  id Int @id @default(autoincrement())

  @@map("PrismaClient")
}
*/
