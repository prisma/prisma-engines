-- tags=cockroachdb

CREATE TABLE "pages" (
    id INT8 PRIMARY KEY DEFAULT unique_rowid(),
    "staticId" INT4 NOT NULL,
    latest INT4 NOT NULL,
    other INT4 NOT NULL,

    CONSTRAINT "full" UNIQUE (other),
    CONSTRAINT "partial" UNIQUE ("staticId") WHERE latest = 1
);


/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "cockroachdb"
  url      = env("DATABASE_URL")
}

model pages {
  id       BigInt @id @default(autoincrement())
  staticId Int
  latest   Int
  other    Int    @unique(map: "full")
}
*/
