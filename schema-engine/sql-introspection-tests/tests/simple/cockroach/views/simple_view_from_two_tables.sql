-- preview_features=views
-- tags=cockroachdb

CREATE TABLE "User" (
    id SERIAL PRIMARY KEY,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NULL
);

CREATE TABLE "Profile" (
    user_id INT PRIMARY KEY,
    introduction TEXT,
    CONSTRAINT Profile_User_fkey FOREIGN KEY (user_id) REFERENCES "User"(id)
);

CREATE VIEW "Schwuser" AS
    SELECT
        u.id,
        CONCAT(u.first_name, ' ', u.last_name) AS name,
        p.introduction
    FROM "User" u
    INNER JOIN "Profile" p ON u.id = p.user_id;




/*
generator js {
  provider = "prisma-client"
  previewFeatures = ["views"]
}

datasource db {
  provider = "cockroachdb"
  url      = env("DATABASE_URL")
}

model Profile {
  user_id      BigInt  @id
  introduction String?
  User         User    @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "profile_user_fkey")
}

model User {
  id         BigInt   @id @default(autoincrement())
  first_name String   @db.String(255)
  last_name  String?  @db.String(255)
  Profile    Profile?
}

/// The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
view Schwuser {
  id           BigInt?
  name         String?
  introduction String?

  @@ignore
}
*/
