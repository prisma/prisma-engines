import { Pool } from 'pg'
import { PgAdapter } from '@jkomyno/prisma-pg-driver-adapter'
import { smokeTest } from './test'
import { bindConnector } from '@jkomyno/prisma-js-connector-utils'

async function pg() {
  const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  const pgPool = new Pool({
    connectionString,
  })

  const adapter = new PgAdapter(pgPool)

  const db = bindConnector(adapter)
  await smokeTest(db, '../prisma/postgres-pg/schema.prisma')
}

pg().catch((e) => {
  console.error(e)
  process.exit(1)
})
