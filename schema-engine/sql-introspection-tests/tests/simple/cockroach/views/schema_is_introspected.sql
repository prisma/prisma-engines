-- preview_features=views
-- schemas=public
-- tags=cockroachdb

CREATE VIEW public."A" AS SELECT 1 AS id;

/*
generator js {
  provider        = "prisma-client-js"
  previewFeatures = ["views"]
}

datasource db {
  provider = "cockroachdb"
  url      = env("DATABASE_URL")
  schemas  = ["public"]
}

/// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
view A {
  id BigInt?

  @@ignore
  @@schema("public")
}
*/
