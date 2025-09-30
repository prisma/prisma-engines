-- tags=cockroachdb

CREATE TABLE "User" (
    id SERIAL PRIMARY KEY,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NULL
);

CREATE VIEW "Schwuser" AS
    SELECT id, first_name, last_name FROM "User";

/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "cockroachdb"
  url      = env("DATABASE_URL")
}

model User {
  id         BigInt  @id @default(autoincrement())
  first_name String  @db.String(255)
  last_name  String? @db.String(255)
}
*/
