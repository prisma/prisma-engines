# Prisma Migrate Architecture

This document will be maintained and expanded over time. It is in the very
early days and mostly contains the Migrate design FAQ.

## FAQ

### Why does Migrate not have down/rollback migrations?

First observation: down migrations serve different purposes in development and
when rolling out changes to production:

**In development**, down migrations are used when:

- You want to iterate on a migration: run the down migration, edit the up
  migration, re-run the up migration.
- You switch branches, and you want to roll back the changes made on the
  branch you are leaving.

In development, we think we already have a better solution. Migrate will tell
you when there is a discrepancy between your migrations and the actual schema
of your dev database, and offer to resolve it for you (currently: by resetting
your development database).

**In production**, down migrations are meant to roll back a bad deployment.

There are a lot of assumptions that need to hold for a down migration to "just work":

- The migration can have partially failed, in which case the full down
  migration will often not work. Did the up migration run to the end? It's far
  from always the case, in case of failure.
- The migration has to be reversible in the first place. It did not drop or
  irreversibly alter anything (table/column) the previous version of the
  application code was using.
- If the migration is invalidating your old application code, you were going to
  have downtime in the first place.
- You have a small enough data set that the rollback will not take hours/bring
  down your application by locking tables. On top of the stress already imposed
  by the bad deployment.
- The down migration actually works. Are your down migrations tested?

In short, down migrations give you a sense of security, but it is often a false
sense of security.

- In production, currently, we will diagnose the problem for you, but rollbacks
  are manual: you use `migrate resolve` to mark the migration as rolled back or
  forward, but the action of rolling back is manual. So it _is_ supported, not
  just as convenient and automated as the rest of the workflows.  Down
  migrations are somewhat rare in real production scenarios, and we are looking
  into better ways to help users recover from failed migrations.

  There are two major avenues for migration tools to be more helpful when a
  deployment fails:

  - Give a short path to recovery that you can take without messing things up
    even more in a panic scenario

  - Guide you towards patterns that can make deploying and recovering from bad
    deployment painless, i.e. forward-only thinking, [expand-and-contract
    pattern](https://www.prisma.io/dataguide/types/relational/expand-and-contract-pattern), etc.

We're looking into how we can best help in these areas. It could very well mean
we'll have down migrations (we're hearing users who want them, and we
definitely want these concerns addressed).

What we recommend instead of relying on down migrations:

- [Adhere to the _expand and contract_ pattern](https://www.prisma.io/dataguide/types/relational/expand-and-contract-pattern)
- Roll-forward in case of failure

Rolling back schema migrations because the corresponding application code has
to be rolled back is an anti-pattern.

The view that guided Migrate development is that _some level_ of awareness of
the migration process (schema and data) will always be required from developers
on large enough projects. What we can do is build tools that help people
getting their migrations right, but it can't be completely automated. There's a
lot more to do, now that we have a stable, production-ready foundation.

### What features in Migrate rely on the shadow database?

The only core feature of Migrate that relies on the shadow database is
**generating migrations**. See `How does Migrate use the shadow database` below
for more details.

### How does Migrate use the shadow database?

The shadow database is the only mechanism by which Migrate can determine what
migrations do. From Migrate's perspective, Migrations are black boxes: we do
not parse SQL, and we want to support all database features that cannot be
represented in the PSL yet. The only way to figure out what the effect of a
migration is is to run it. This is necessary when we need to figure out the
current state of a migrations history.

Here is how we generate a new migration:

- If the `shadowDatabaseUrl` datasource param is set in Prisma schema, connect
  to that database and try to erase all schema. Otherwise create an empty
  shadow database.
- Apply all the migrations in the migrations directory to the shadow database.
  Introspect its schema: this is the schema we will assume as a _starting
  point_ for the next migration.
- Calculate the _expected_ database schema from the contents of the Prisma
  schema.
- Diff the _starting point_ with the _expected_ schemas: this diff is the next
  migration that we will write to a file.

This logic is implemented in the migration connectors.

![image](https://user-images.githubusercontent.com/6864947/141001419-3d0fb4ff-e2e3-4e95-bb67-255dc66b7acb.png)

### Can `prisma migrate deploy` ask to reset the database? Does it use the shadow database?

No. `prisma migrate deploy` will never use a shadow database, and it will never reset
your database.

On a high level, migrate deploy _exclusively_:

- Figures out which migrations have been run and which migrations have not, by
  looking at the `_prisma_migrations` table in the target database.
- Run the migrations that have not been applied yet, in chronological order.

`prisma migrate deploy` is the command meant to be used for _unattended_
migrations (as part of CI). As such, it should be as reliable, predictable and
deterministic as possible.

### Why does `prisma migrate deploy` not warn when a migration is in the migrations table but not in the migrations folder?

We don't want to warn if an already applied migration is missing from the
migration directory because it would prevent squashing migrations and
deployment from not-properly-rebased branches.

In general, by design, `deploy` errs on the side of not standing in the way of
deploying your migrations.

### Why does `prisma migrate deploy` not detect drift?

We don't detect drift because we want to keep the deployment path light and
simple, and because within the current Migrate architecture, we would need a
shadow database for that. Many people would not be comfortable with
creating/using temporary databases being on the deployment path.

And as stated in the previous question, in general, by design,
`deploy` errs on the side of not standing in the way of deploying your
migrations.

### What happens when a migration fails in `prisma migrate deploy`?

If a migration fails during deployment, you will see the error. Then `prisma
migrate status` and `prisma migrate deploy` commands (in subsequent runs) will
show you the failed state with the error message when you run them again.

It's then your responsibility to fix what failed, and mark the migration as
applied or rolled back with `prisma migrate resolve`, so you can deploy
migrations again.

Prisma Migrate does not offer much help at that last stage, but this is
something we are working on defining and prioritizing. See [this
issue](https://github.com/prisma/prisma/issues/10127).

Also see the [public
documentation](https://www.prisma.io/docs/guides/database/production-troubleshooting)
on this topic.

### Why does Migrate not do data migrations in TypeScript?

One important reason is that we believe data and schema migrations should be
separated, they should not run at the same time (see resource 1).

One other assumption with Prisma Migrate is that since we are an abstraction
over the database, and support many of them, we'll never cover 100% of the
features (e.g. check constraints, triggers, stored procedures, etc.), so we
have to take the database as the source of truth and let users define what we
don't represent in the Prisma Schema. On SQL databases, we let you write raw
SQL migrations for example, so you have full control if you need it.

Migrations are high stakes on production apps, and they should be as
straightforward and deterministic to apply as possible. Every extra layer of
abstraction is a risk.

Resources:

1. https://thoughtbot.com/blog/data-migrations-in-rails

### Why does `migrate dev` constantly want to reset my database? Could it not simply roll back to the desired state?

The development flow in migrate (the `migrate dev` command) is quite pedantic:
it will check things like migrations missing from your migrations folder but
already applied to the dev database, or migrations that were modified since
they were applied (via a checksum) and guide you towards resolving the problem.
That can happen because of merges, but even more commonly when you are just
switching branches locally, or editing migrations.

We could absolutely make the database match your models (equivalent to db push)
whenever we detect drift. However, there is a design constraint that makes this
undesirable: the migrations can contain arbitrary SQL, including database
features that cannot be represented in the Prisma schema and that the Prisma
engines do not know about, like check constraints and views.

Since these can't be diffed nor rolled back, the only way migrate has to make
sure that the database schema state actually matches the migrations in the
migrations folder is to reset the database and reapply them.

The main sources of drift in development would be 1. switching branches, and
more generally version control with collaborators, 2. iterating on/editing of
migrations, 3. manual fiddling with the database.

### Why does migrate use migration files? Why not go fully declarative?

Declarative migrations are a huge time saver — even when you write a lot of SQL
for schema migrations, and you still often have to look up the exact syntax for
DDL statements, or deal with really obscure errors from the database when you
use the wrong type of quotes marks.

However, there are downsides to a purely declarative approach, where you only
define the target schema:

- This does not let you control _how_ the migration is performed. Sometimes you
  need more control. Sometimes schema migrations have to be broken up into
  multiple steps deployed separately, to leave time and space to migrate your
  code and your data in between.
- Some operations are just not possible in general in a declarative way: adding
  a non-nullable column without a default to a table with existing rows, for
  example. Maybe you want to create the new column as nullable, populate it
  with data computed from other columns/tables, then make it non-nullable. This
  is something a tool can't guess for you.
- These migrations are not always reversible: you dropped a unique constraint,
  now you want to add it back, but your data has duplicates. Or you dropped a
  table and the foreign keys pointing to it, and now you want to restore it,
  but you lost the data, so the foreign keys can't be restored.
- When, like in the last two points above, you — the author of the migration —
  have to make decisions, declarative tools often don't have a good answer to
  when these decisions are made (development time? apply time? are unattended
  migrations in CI possible, in that case?) and where they are persisted, in
  the absence of a folder containing the migrations history.
- Renamings are tricky. You usually do not want them on databases with real
  production traffic, but in the early stages of a project, they are nice to
  have.

The main advantage of declarative migrations is the ease of use: you don't have
to write migrations. It's a huge time saver. At the same time, you do want
control and reproducibility of the migrations you apply: you should be able to
tweak the migrations, persist them into a file and have them reviewed along
with code changes. So a hybrid approach where you still have SQL migration
files, but they are generated for you, and you can optionally edit them, seems
to be a best-of-both-worlds solution.

- You still get the convenience of declarative migrations: if you are fine with
  the automatically generated migration, just commit it and you are done.
- You frontload the decisions about data loss and tough migrations: you get to
  apply and test the migration script on your local database, and get it
  reviewed through the normal process with the rest of your code. It is then
  applied exactly as it was written.
- With each new migration, you start with a SQL file that already performs the
  schema changes you wanted. It's easier to adjust something that is mostly
  there, than remembering how to write the whole thing. We expect users to
  tinker with the migrations.
- Not only do you get the generated SQL script, but the migration engine does
  know what changes are potentially destructive or impossible, or things that
  could go wrong with large amounts of data, and it can document that in the
  migration script directly for you to review and make decisions about.
- The tool acknowledges it won't be able to declaratively handle everything. If
  you want to tweak row-level security policies in your migration scripts, you
  absolutely can.
- You do want to know exactly what changeset was applied to the database, when
  things go wrong, and when comparing different deployments/environments.

The workflow of working with temporary databases and introspecting it to
determine differences between schemas seems to be pretty common, this is for
example what skeema does. This is also what migrate will do locally.

### Why does Migrate not run migrations in a transaction by default?

**Determinism/repeatability**. It has been proposed that migrate automatically
wraps migrations in development where possible, but it would change how
migrations work between development and production, and this contrary to our
belief in _reproducibility_.

**Flexibility** If we do not wrap in a transaction by default, users have the
option to add a BEGIN; and a COMMIT; to the migrations they want wrapped in a
transaction. If we did wrap _implicitly_, we would need an extra opt-out
mechanism if we want users to have the option to opt out.

**Consistency**. We would not be able to do this on all databases we support,
leading to different expectations. It's not possible on MySQL for example.

**Performance**. Large migrations will be much heavier if wrapped in a
transaction (locking, additional state to maintain for the database...).

That said, in most cases, if you have the option, it's better to wrap your
migrations, or part of your migrations in transactions. It wouldn't be
backward-compatible for migrate to implicitly wrap migrations in transactions,
but adding or removing `BEGIN;` and `COMMIT;` statements in _new_ migrations is
always ok, whether done by Migrate or manually. A migration history should have
a consistent transaction-or-no-transaction default. With an option somewhere,
you could break your existing migrations without editing them by just changing
that option. An option to generate (or not) a `BEGIN;` and a `COMMIT;` for new
migrations would be conceivable.

The migration engine should blissfully ignore that problem when actually
applying migrations (we want that code to stay as simple as possible because
it's critical).

For reproducibility, we always want to run exactly the same migration in dev
and production, in the same way. "Why would we have a failing migration on
production if it worked on development?" is not a valid objection to treating
migrations differently (e.g. with transactions). Small differences between dev
and prod databases, data migrations triggering unique constraint
violations/foreign key errors/nullability errors, failing type casts, etc. can
cause the same migration to fail in one environment and succeed in another.
