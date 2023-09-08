-- tags=mysql
-- exclude=vitess

-- Excluding Vitess because of foreign
-- keys.

-- This table has no unique criteria, it will be ignored.
CREATE TABLE `pastry` (
    `name` VARCHAR(50),
    INDEX `myidx`(`name`)
);

CREATE TABLE `topping` (
    `toppingName` VARCHAR(120) PRIMARY KEY,
    `pastryName` VARCHAR(50),

    CONSTRAINT `pastryfk` FOREIGN KEY (`pastryName`) REFERENCES `pastry`(`name`) ON DELETE RESTRICT ON UPDATE RESTRICT
);


/*
generator js {
  provider = "prisma-client-js"
}

datasource db {
  provider = "mysql"
  url      = env("DATABASE_URL")
}

/// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
model pastry {
  name    String?   @db.VarChar(50)
  topping topping[]

  @@index([name], map: "myidx")
  @@ignore
}

model topping {
  toppingName String  @id @db.VarChar(120)
  pastryName  String? @db.VarChar(50)
  pastry      pastry? @relation(fields: [pastryName], references: [name], onDelete: Restrict, onUpdate: Restrict, map: "pastryfk") @ignore

  @@index([pastryName], map: "pastryfk")
}
*/
