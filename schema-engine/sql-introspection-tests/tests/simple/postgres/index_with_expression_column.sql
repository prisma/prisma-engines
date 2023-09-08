-- tags=postgres
-- exclude=cockroachdb

CREATE TABLE communication_channels (
    id bigint NOT NULL,
    path character varying(255) NOT NULL,
    path_type character varying(255) DEFAULT 'email'::character varying NOT NULL,
    position integer,
    user_id bigint NOT NULL,
    pseudonym_id bigint,
    bounce_count integer DEFAULT 0,
    confirmation_code character varying(255)
);

/*
now create the indexes with expression columns
*/

CREATE INDEX index_communication_channels_on_path_and_path_type ON communication_channels (lower((path)::text), path_type);
CREATE UNIQUE INDEX index_communication_channels_on_user_id_and_path_and_path_type ON communication_channels (user_id, lower((path)::text), path_type);
CREATE INDEX index_communication_channels_on_confirmation_code ON communication_channels (confirmation_code);

/*
generator js {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

/// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
/// This model contains an expression index which requires additional setup for migrations. Visit https://pris.ly/d/expression-indexes for more info.
model communication_channels {
  id                BigInt
  path              String  @db.VarChar(255)
  path_type         String  @default("email") @db.VarChar(255)
  position          Int?
  user_id           BigInt
  pseudonym_id      BigInt?
  bounce_count      Int?    @default(0)
  confirmation_code String? @db.VarChar(255)

  @@index([confirmation_code], map: "index_communication_channels_on_confirmation_code")
  @@ignore
}
*/
