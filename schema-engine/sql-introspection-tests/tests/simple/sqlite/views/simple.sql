-- preview_features=views
-- tags=sqlite

CREATE TABLE A (
    id INT NOT NULL PRIMARY KEY,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NULL
);

CREATE VIEW B AS SELECT id, first_name, last_name FROM A;


/*
generator js {
  provider        = "prisma-client-js"
  previewFeatures = ["views"]
}

datasource db {
  provider = "sqlite"
  url      = env("DATABASE_URL")
}

model A {
  id         Int     @id
  first_name String
  last_name  String?
}

/// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
view B {
  id         Int?
  first_name String?
  last_name  String?

  @@ignore
}
*/
