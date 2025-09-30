-- tags=mysql8
-- exclude=vitess

CREATE TABLE customers
(
    id       INT AUTO_INCREMENT PRIMARY KEY,
    custinfo JSON,
    -- We do not render these yet.
    INDEX zips ((CAST(custinfo -> '$.zipcode' AS UNSIGNED ARRAY)))
);

/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "mysql"
  url      = env("DATABASE_URL")
}

model customers {
  id       Int   @id @default(autoincrement())
  custinfo Json?
}
*/
