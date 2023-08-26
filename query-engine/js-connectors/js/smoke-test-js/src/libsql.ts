import { createLibsqlConnector } from '@jkomyno/prisma-libsql-js-connector'
import { smokeTest } from './test'

async function pg() {
  const connectionString = `${process.env.JS_LIBSQL_DATABASE_URL as string}`
  const authToken = `${process.env.JS_LIBSQL_TOKEN as string}`

  const db = createLibsqlConnector({
    url: connectionString,
    token: authToken
  })

  await smokeTest(db, '../prisma/sqlite-libsql/schema.prisma')
}

pg().catch((e) => {
  console.error(e)
  process.exit(1)
})
