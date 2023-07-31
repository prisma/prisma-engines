-- CreateTable
CREATE TABLE `some_users` (
    `id` INTEGER NOT NULL AUTO_INCREMENT,
    `firstname` VARCHAR(32) NOT NULL,
    `lastname` VARCHAR(32) NOT NULL,

    PRIMARY KEY (`id`)
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- CreateTable
CREATE TABLE `Parent` (
    `p` VARCHAR(191) NOT NULL,
    `p_1` VARCHAR(191) NOT NULL,
    `p_2` VARCHAR(191) NOT NULL,
    `non_unique` VARCHAR(191) NULL,
    `id` VARCHAR(191) NOT NULL,

    UNIQUE INDEX `Parent_p_key`(`p`),
    UNIQUE INDEX `Parent_p_1_p_2_key`(`p_1`, `p_2`),
    PRIMARY KEY (`id`)
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- CreateTable
CREATE TABLE `Child` (
    `c` VARCHAR(191) NOT NULL,
    `c_1` VARCHAR(191) NOT NULL,
    `c_2` VARCHAR(191) NOT NULL,
    `parentId` VARCHAR(191) NULL,
    `non_unique` VARCHAR(191) NULL,
    `id` VARCHAR(191) NOT NULL,

    UNIQUE INDEX `Child_c_key`(`c`),
    UNIQUE INDEX `Child_parentId_key`(`parentId`),
    UNIQUE INDEX `Child_c_1_c_2_key`(`c_1`, `c_2`),
    PRIMARY KEY (`id`)
) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
