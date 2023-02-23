-- tags=mysql

CREATE TABLE A (
    id INT PRIMARY KEY,
    val ENUM('a', 'b')
);

CREATE VIEW B AS SELECT id, val from A;

/*
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "mysql"
  url      = "env(TEST_DATABASE_URL)"
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
