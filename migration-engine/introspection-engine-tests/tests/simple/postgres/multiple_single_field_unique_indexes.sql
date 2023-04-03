-- tags=postgres
-- exclude=cockroachdb

CREATE TABLE mymodel (
    id UUID PRIMARY KEY,
    thefield TEXT
);

CREATE UNIQUE INDEX unq2 ON mymodel(thefield);
CREATE UNIQUE INDEX unq3 ON mymodel(thefield);
CREATE UNIQUE INDEX unq1 ON mymodel(thefield);


/*
generator js {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

model mymodel {
  id       String  @id @db.Uuid
  thefield String? @unique(map: "unq1")
}
*/
