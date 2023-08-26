import { createLibsqlConnector } from '@jkomyno/prisma-libsql-connector'
import { smokeTest } from './test'

async function pg() {
  const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  const db = createLibsqlConnector({
    url: connectionString,
  })

  await smokeTest(db, '../prisma/sqlite-libsql/schema.prisma')
}

pg().catch((e) => {
  console.error(e)
  process.exit(1)
})
