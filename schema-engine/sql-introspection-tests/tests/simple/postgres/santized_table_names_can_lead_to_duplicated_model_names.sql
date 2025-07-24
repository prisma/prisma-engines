-- tags=postgres
-- exclude=cockroachdb

-- Models can get renamed due to unsupported characters in their names and end up with the same name.
-- This is technically an invalid schema but something left for the user to fix manually.

CREATE TABLE "nod?es"(id serial primary key);
CREATE TABLE "nod!es"(
    node_a int NOT NULL,
    node_b int NOT NULL,
    CONSTRAINT _nodes_node_a_fkey FOREIGN KEY(node_a) REFERENCES "nod?es"(id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT _nodes_node_b_fkey FOREIGN KEY(node_b) REFERENCES "nod?es"(id) ON DELETE CASCADE ON UPDATE CASCADE
);

/*
generator js {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

/// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
model public_nod_es {
  node_a                       Int
  node_b                       Int
  nod_es_nod_es_node_aTonod_es public_nod_es @relation("nod_es_node_aTonod_es", fields: [node_a], references: [id], onDelete: Cascade, map: "_nodes_node_a_fkey")
  nod_es_nod_es_node_bTonod_es public_nod_es @relation("nod_es_node_bTonod_es", fields: [node_b], references: [id], onDelete: Cascade, map: "_nodes_node_b_fkey")

  @@map("nod!es")
  @@ignore
}

model public_nod_es {
  id                           Int             @id @default(autoincrement())
  nod_es_nod_es_node_aTonod_es public_nod_es[] @relation("nod_es_node_aTonod_es") @ignore
  nod_es_nod_es_node_bTonod_es public_nod_es[] @relation("nod_es_node_bTonod_es") @ignore

  @@map("nod?es")
}
*/
