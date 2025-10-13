-- preview_features=views
-- schemas=public
-- tags=cockroachdb

CREATE VIEW public."A" AS SELECT 1 AS id;

/*
generator js {
  provider = "prisma-client"
  previewFeatures = ["views"]
}

datasource db {
  provider = "cockroachdb"
  url      = env("DATABASE_URL")
  schemas  = ["public"]
}

view A {
  id BigInt?

  @@ignore
  @@schema("public")
}
*/
