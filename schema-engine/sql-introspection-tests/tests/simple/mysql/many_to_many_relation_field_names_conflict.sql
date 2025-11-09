-- tags=mysql
-- exclude=vitess

CREATE TABLE `User` (
    id INTEGER AUTO_INCREMENT PRIMARY KEY
);

CREATE TABLE `_Friendship` (
    `A` INTEGER NOT NULL,
    `B` INTEGER NOT NULL,

    FOREIGN KEY (`A`) REFERENCES `User`(`id`),
    FOREIGN KEY (`B`) REFERENCES `User`(`id`)
);

CREATE UNIQUE INDEX `_Friendship_AB_unique` ON `_Friendship`(`A`, `B`);
CREATE INDEX `_Friendship_B_index` ON `_Friendship`(`B`);

CREATE TABLE `_Frenemyship` (
    `A` INTEGER NOT NULL,
    `B` INTEGER NOT NULL,
    FOREIGN KEY (`A`) REFERENCES `User`(`id`),
    FOREIGN KEY (`B`) REFERENCES `User`(`id`)
);

CREATE UNIQUE INDEX `_Frenemyship_AB_unique` ON `_Frenemyship`(`A`, `B`);
CREATE INDEX `_Frenemyship_B_index` ON `_Frenemyship`(`B`);




/*
generator js {
  provider = "prisma-client"
}

datasource db {
  provider = "mysql"
}

model User {
  id                 Int    @id @default(autoincrement())
  User_Frenemyship_A User[] @relation("Frenemyship")
  User_Frenemyship_B User[] @relation("Frenemyship")
  User_Friendship_A  User[] @relation("Friendship")
  User_Friendship_B  User[] @relation("Friendship")
}
*/
