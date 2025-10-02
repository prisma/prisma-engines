-- tags=postgres
-- exclude=cockroachdb

-- Test repro for https://github.com/prisma/prisma/issues/14389

CREATE EXTENSION intarray;

CREATE TABLE test (
    big_data BOOLEAN PRIMARY KEY
);

CREATE INDEX futureproof ON test(big_data);



/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

model test {
  big_data Boolean @id

  @@index([big_data], map: "futureproof")
}
*/
