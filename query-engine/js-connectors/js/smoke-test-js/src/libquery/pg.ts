import { createPgConnector } from '@jkomyno/prisma-pg-js-connector'
import { smokeTestLibquery } from './libquery' 

async function pg() {
  const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  const db = createPgConnector({
    url: connectionString,
  })

  await smokeTestLibquery(db, '../../prisma/postgres/schema.prisma')
}

pg().catch((e) => {
  console.error(e)
  process.exit(1)
})
