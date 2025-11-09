-- preview_features=views
-- schemas=public
-- tags=cockroachdb

CREATE VIEW public."A" AS SELECT 1 AS id;


/*
generator js {
  provider        = "prisma-client"
  previewFeatures = ["views"]
}

datasource db {
  provider = "cockroachdb"
  schemas  = ["public"]
}

view A {
  id BigInt?

  @@schema("public")
}
*/
