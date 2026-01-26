-- tags=postgres

CREATE TABLE "User" (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL,
    status TEXT NOT NULL
);

CREATE UNIQUE INDEX "email_active_unique" ON "User" (email) WHERE (status = 'active');


/*
generator js {
  provider        = "prisma-client"
  previewFeatures = ["partialIndexes"]
}

datasource db {
  provider = "postgresql"
}

model User {
  id     Int    @id
  email  String @unique(map: "email_active_unique", where: raw("(status = 'active'::text)"))
  status String
}
*/
