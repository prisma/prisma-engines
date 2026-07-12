-- tags=mssql

CREATE TABLE [dbo].[a] (
    aid INT IDENTITY,
    acol INT NOT NULL
);

CREATE TABLE [dbo].[b] (
    bid INT IDENTITY,
    bcol INT NOT NULL
);

/* The first index will be the primary key of a. */
ALTER TABLE a ADD CONSTRAINT a_pkey PRIMARY KEY (aid);

/*
    The second index will be the index on b with an included (non-key) column.

    The bcol column should not be included in the index in the introspected
    schema, because it is not part of the key (the indexed columns). It was
    previously erroneously included in the primary key of `a` because its
    key_ordinal was 0. That caused crashes.

    See the official docs on included columns:
    https://docs.microsoft.com/en-us/sql/t-sql/statements/create-index-transact-sql?view=sql-server-ver16#include-column---n--
*/
CREATE UNIQUE INDEX bidx ON b (bid) INCLUDE (bcol);



/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "sqlserver"
}

model a {
  aid  Int @id @default(autoincrement())
  acol Int
}

model b {
  bid  Int @unique(map: "bidx") @default(autoincrement())
  bcol Int
}
*/
