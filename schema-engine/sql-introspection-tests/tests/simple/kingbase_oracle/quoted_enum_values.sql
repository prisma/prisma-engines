-- tags=KingbaseOracle

CREATE TYPE mood AS ENUM ('happy', 'needs-review', '123');

CREATE TABLE enum_defaults (
    id INTEGER PRIMARY KEY,
    current_mood mood NOT NULL DEFAULT 'happy'
);
/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "kingbase-oracle"
}

model enum_defaults {
  id           Int  @id
  current_mood mood @default(happy)
}

enum mood {
  happy
  needs_review @map("needs-review")
  // 123 @map("123")
}
*/
