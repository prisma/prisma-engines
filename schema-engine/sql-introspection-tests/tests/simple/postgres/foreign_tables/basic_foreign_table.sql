-- tags=postgres
-- exclude=cockroachdb

-- We have to enable the extension.
-- Run all these commands on postgres15!
CREATE EXTENSION postgres_fdw;

-- Creates a server object to the database.
CREATE SERVER foreign_server
--            ^^^^^^^^^^^^^^ name of the server object
FOREIGN DATA WRAPPER postgres_fdw
OPTIONS (host 'postgres14', port '5432', dbname 'postgres');
--       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ connection options

CREATE USER MAPPING
FOR postgres
--  ^^^^^^^^ map this user on this server
SERVER foreign_server
--     ^^^^^^^^^^^^^^ on this server object we created above
OPTIONS (user 'postgres', password 'prisma');
--       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ login detains in the other server

CREATE FOREIGN TABLE bar (
--                   ^^^ name of the table on this server
  id SERIAL NOT NULL
)
SERVER foreign_server
--     ^^^^^^^^^^^^^^ again our server object
OPTIONS (schema_name 'public', table_name 'bar');
--                   ^^^^^^^^ name of the schema in the remote server
--                                        ^^^^^ name of the table in the remote server

/*
generator js {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}
*/
