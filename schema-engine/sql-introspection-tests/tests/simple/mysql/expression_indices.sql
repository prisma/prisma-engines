-- tags=mysql8
-- exclude=vitess

CREATE TABLE customers
(
    id       INT AUTO_INCREMENT PRIMARY KEY,
    custinfo JSON,
    INDEX zips ((CAST(custinfo -> '$.zipcode' AS UNSIGNED ARRAY)))
);

/*
generator js {
  provider = "prisma-client-js"
}

datasource db {
  provider = "mysql"
  url      = env("DATABASE_URL")
}

/// This table contains multi-value indices, which are not yet fully supported. Visit https://pris.ly/d/mysql-multi-row-index for more info.
model customers {
  id       Int   @id @default(autoincrement())
  custinfo Json?
}
*/
