
# Implementation Documentation: Mapping a Prisma Schema to a SQL Database

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
- [Mapping Models](#mapping-models)
  - [Mapping a model with a single Id field](#mapping-a-model-with-a-single-id-field)
  - [Mapping a model with a compound Id field](#mapping-a-model-with-a-compound-id-field)
  - [Mapping a model with singular scalar fields](#mapping-a-model-with-singular-scalar-fields)
  - [Mapping a model with scalar list fields](#mapping-a-model-with-scalar-list-fields)
  - [Mapping a model with a unique field](#mapping-a-model-with-a-unique-field)
  - [Mapping a model with a compound unique field](#mapping-a-model-with-a-compound-unique-field)
- [Mapping Enums](#mapping-enums)
- [Mapping Default Values](#mapping-default-values)
- [Mapping Relations](#mapping-relations)
  - [Mapping a one to one relation](#mapping-a-one-to-one-relation)
  - [Mapping a one to many relation](#mapping-a-one-to-many-relation)
  - [Mapping a many to many relation](#mapping-a-many-to-many-relation)
  - [Mapping Self Relations and Ambigous Relations](#mapping-self-relations-and-ambigous-relations)
  - [Mapping Compound Foreign Keys](#mapping-compound-foreign-keys)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

The schema of the underlying database can be deducted from a [Prisma schema](https://github.com/prisma/specs/tree/master/schema). This document describes how this is done for SQL databases.


## Mapping Models

### Mapping a model with a single Id field
Prisma Schema Example with CUID:
```groovy
model Blog {
    id        String @id @default(cuid())
}
```

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."Blog" (
  "id" text NOT NULL  ,
  PRIMARY KEY ("id")
);

```
Prisma Schema Example with Int Id:
```groovy
model Blog {
    id  Int @id
}
```

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."Blog" (
  "id" SERIAL,
  PRIMARY KEY ("id")
);
```

Prisma Schema Example with UUID:
```groovy
model Blog {
    id        String @id @default(uuid())
}
```

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."Blog" (
  "id" text NOT NULL  ,
  PRIMARY KEY ("id")
);
```

🚨 The `cuid()` and `uuid()` expressions have no manifestation in the database, and therefore can later not be inferred. 

### Mapping a model with a compound Id field
Prisma Schema Example:
```groovy
 model Blog {
    id Int
    authorId String
    @@id([id, authorId])
}
```


This should be the resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."Blog" (
  "id" integer NOT NULL  ,
  "authorId" text NOT NULL  ,
  PRIMARY KEY ("id", "authorId")
);
```

### Mapping a model with singular scalar fields

Prisma Schema Example:
```groovy
model Blog {
    id        String @id @default(cuid())
    title     String
    viewCount Int
    isPremium Boolean
}
```

A model in the Prisma schema always maps to a table in the underlying SQL database. Each scalar field is mapped to a column by mapping the field type to a type in the SQL database. [TODO: Insert Reference for the Type Mapping Reference here]

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."Blog" (
  "id" text NOT NULL  ,
  "title" text NOT NULL DEFAULT '' ,
  "viewCount" integer NOT NULL DEFAULT 0 ,
  PRIMARY KEY ("id")
);
```

### Mapping a model with scalar list fields

Prisma Schema Example:
```groovy
model User {
    id Int @id
    ints Int[]
}
```

A scalar list field in PSL is mapped to a separate table for the values. The name follows the `Model_fieldname` convention. The important columns contained are nodeId for the specific node a value is associated with and value for the actual value. Position is generated and should allow fancy operations like ordered inserts, but is not used. 

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."User" (
  "id" SERIAL,
  PRIMARY KEY ("id")
);

CREATE TABLE "test"."User_ints" (
  "nodeId" integer   REFERENCES "test"."User"("id") ON DELETE CASCADE,
  "position" integer NOT NULL  ,
  "value" integer NOT NULL  ,
  PRIMARY KEY ("nodeId","position")
);
```

### Mapping a model with a unique field

```groovy
model Blog {
    id        String @id @default(cuid())
    title     String
    slug      String @unique
}
```

For a field that is annotated with `@unique` a unique index is created in the underlying database.

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."Blog" (
  "id" text NOT NULL  ,
  "slug" text NOT NULL DEFAULT '' ,
  "title" text NOT NULL DEFAULT '' ,
  PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "Blog.slug" ON "test"."Blog"("slug")
```

### Mapping a model with a compound unique field

```groovy
model User {
    id Int @id
    firstname String
    lastname String
    @@unique([firstname, lastname], name: "test")
}
```

For the block attribute `@@unique` a unique index is created in the underlying database.

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."User" (
  "firstname" text NOT NULL DEFAULT '' ,
  "id" SERIAL,
  "lastname" text NOT NULL DEFAULT '' ,
  PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "test" ON "test"."User"("firstname","lastname")
```


## Mapping Enums

```groovy
model User {
    id Int @id
    enum TestEnum
}

enum TestEnum{
  A
  B
}
```

We handle the validation of Enum values in the core, and place no constraints on the db column.  

```sql
CREATE TABLE "test"."User" (
  "enum" text NOT NULL DEFAULT 'A' ,
  "id" SERIAL,
  PRIMARY KEY ("id")
);
```

🚨 We do not use any DB capabilities for enums, therefore when introspecting again, this is just a String field for us. 



## Mapping Default Values 

```groovy
model User {
  id       Int       @id
  string   String    @default("test")
  date     DateTime  @default(now())
}
```

```sql
CREATE TABLE "test"."User" (
  "date" timestamp(3) NOT NULL DEFAULT '1970-01-01 00:00:00' ,
  "id" SERIAL,
  "string" text NOT NULL DEFAULT 'test' ,
  PRIMARY KEY ("id")
);
```

🚨 DateTime default values don't yet work. The `now()` expression should translate to something like `CURRENT_TIMESTAMP`. 




## Mapping Relations

### Mapping a one to one relation

There are four potential cases for 1to1 relations when it comes to fields being required. But two of them are always resulting in the same SQL structure. 

Both sides required:
```groovy
model User {
    id      String @id @default(cuid())
    address Address @relation(references: [id])
}
model Address {
    id   String @id @default(cuid())
    user User
}
```

One side required:
```groovy
model User {
    id      String @id @default(cuid())
    address? Address @relation(references: [id])
}
model Address {
    id   String @id @default(cuid())
    user User
}
```

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."User" (
  "id" text NOT NULL  ,
  PRIMARY KEY ("id")
);

CREATE TABLE "test"."Address" (
  "id" text NOT NULL  ,
  PRIMARY KEY ("id")
);

ALTER TABLE "test"."User" ADD COLUMN "address" text NOT NULL   REFERENCES "test"."Address"("id") ON DELETE SET NULL;
```

Other side required:
```groovy
model User {
    id      String @id @default(cuid())
    address Address @relation(references: [id])
}
model Address {
    id   String @id @default(cuid())
    user User?
}
```

Neither side required:
```groovy
model User {
    id      String @id @default(cuid())
    address? Address @relation(references: [id])
}
model Address {
    id   String @id @default(cuid())
    user User?
}
```

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."User" (
  "id" text NOT NULL  ,
  PRIMARY KEY ("id")
);

CREATE TABLE "test"."Address" (
  "id" text NOT NULL  ,
  PRIMARY KEY ("id")
);

ALTER TABLE "test"."User" ADD COLUMN "address" text   REFERENCES "test"."Address"("id") ON DELETE SET NULL;
```

For a one to one relation the user must specify the side where the foreign key gets stored by annotating the field with `@relation(references: [id])`. This annotated field is mapped to a column with a foreign key constraint. This foreign key constraint is set to null if the referenced row is deleted.

🚨 In the 1To1 case we should probably put a unique constraint on the address column, otherwise we can't infer that it is a 1To1 when introspecting again.
We also loose the information which case it is exactly in the SQL structure. 

### Mapping a one to many relation
```groovy
model Blog {
    id    String @id @default(cuid())
    title String
    posts Post[]
}

model Post {
    id   String @id @default(cuid())
    blog Blog
}
```

For a one to many relation the user must not specify where the foreign key gets stored. The relation field that is singular is mapped to a column with a foreign key constraint. This foreign key constraint is set to null if the referenced row is deleted.

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."Blog" (
  "id" text NOT NULL  ,
  "title" text NOT NULL DEFAULT '' ,
  PRIMARY KEY ("id")
);

CREATE TABLE "test"."Post" (
  "id" text NOT NULL  ,
  PRIMARY KEY ("id")
);

ALTER TABLE "test"."Post" ADD COLUMN "blog" text   REFERENCES "test"."Blog"("id") ON DELETE SET NULL;
```
 

### Mapping a many to many relation
```groovy
model Blog {
    id         String @id @default(cuid())
    title      String
    categories Category[]
}
model Category {
    id    String @id @default(cuid())
    name  String
    blogs Blog[]
}
```

A many to many relation is mapped to the database by creating an intermediate join table. The name of the table is the name of relation prefixed with an underscore. This table contains two columns called `A` and `B` that contain the foreign keys. The column `A` contains the foreign key for the model with the lexicographically lower name. If the referenced row is deleted the row in the join table is deleted as well. A compound unique index is created for those columns to ensure that an association between 2 rows cannot be established twice.

Resulting SQL Example (Postgres):
```sql
CREATE TABLE "test"."Blog" (
  "id" text NOT NULL  ,
  "title" text NOT NULL DEFAULT '' ,
  PRIMARY KEY ("id")
);

CREATE TABLE "test"."Category" (
  "id" text NOT NULL  ,
  "name" text NOT NULL DEFAULT '' ,
  PRIMARY KEY ("id")
);

CREATE TABLE "test"."_BlogToCategory" (
  "A" text   REFERENCES "test"."Blog"("id") ON DELETE CASCADE,
  "B" text   REFERENCES "test"."Category"("id") ON DELETE CASCADE
);

CREATE UNIQUE INDEX "_BlogToCategory_AB_unique" ON "test"."_BlogToCategory"("A","B")
```

### Mapping Self Relations and Ambigous Relations
```groovy
model Employee {
  employee_id       Int       @id
  reporting_to      Employee?  @relation("Reporting")
  gets_reports_from Employee[] @relation("Reporting")
  recruited_by      Employee?  @relation("Recruiting")
  recruited         Employee[] @relation("Recruiting")
}
```

```sql
CREATE TABLE "test"."Employee" (
  "employee_id" SERIAL,
  PRIMARY KEY ("employee_id")
);

ALTER TABLE "test"."Employee" ADD COLUMN "recruited_by" integer   REFERENCES "test"."Employee"("employee_id") ON DELETE SET NULL,
ADD COLUMN "reporting_to" integer   REFERENCES "test"."Employee"("employee_id") ON DELETE SET NULL;
```

🚨 We do not persist names for backrelationfields or relation names. This information is then lost when introspecting. 

### Mapping Compound Foreign Keys
```groovy
 model User {
    id Int @id
    post Post? 
    firstname String
    lastname String
}

model Post {
    id Int @id
    user User @relation(references:[firstname, lastname])
}
```

```sql
CREATE TABLE "test"."User" (
  "firstname" text NOT NULL DEFAULT '' ,
  "id" SERIAL,
  "lastname" text NOT NULL DEFAULT '' ,
  PRIMARY KEY ("id")
);

CREATE TABLE "test"."Post" (
  "id" SERIAL,
  PRIMARY KEY ("id")
);

ALTER TABLE "test"."Post" ADD COLUMN "user" integer   REFERENCES "test"."User"("id") ON DELETE SET NULL;
```

🚨 The SQL structure here is completely wrong, and PSL atm also does not specify this case. The annotation should look like this: `@relation(from:[User_firstname, User_lastname], references:[firstname, lastname])`, but currently the from field is not specced. 

The correct SQL should probably look like this:
```sql
CREATE TABLE "test"."User" (
  "firstname" text NOT NULL DEFAULT '' ,
  "id" SERIAL,
  "lastname" text NOT NULL DEFAULT '' ,
  PRIMARY KEY ("id")
);

CREATE TABLE "test"."Post" (
  "id" SERIAL,
  "User_firstname" text NOT NULL DEFAULT '' ,
  "User_lastname" text NOT NULL DEFAULT '' ,
  PRIMARY KEY ("id")
  FOREIGN KEY ("User_firstname", "User_lastname") REFERENCES "test"."User" ("firstname", "lastname"),
);
```
