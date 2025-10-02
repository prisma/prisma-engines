-- tags=mysql
-- exclude=vitess

CREATE TABLE A (
    id INT PRIMARY KEY,
    val ENUM('a', 'b')
);

CREATE VIEW B AS SELECT id, val from A;



/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "mysql"
  url      = env("DATABASE_URL")
}

model A {
  id  Int    @id
  val A_val?
}

enum A_val {
  a
  b
}
*/
