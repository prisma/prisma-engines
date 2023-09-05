import pg from 'pg'
import { PrismaPostgres } from '@jkomyno/prisma-adapter-pg'
import { bindAdapter } from '@jkomyno/prisma-adapter-utils'
import { smokeTestLibquery } from './libquery.js'

async function main() {
  const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  const pool = new pg.Pool({ connectionString })
  const adapter = new PrismaPostgres(pool)
  const driverAdapter = bindAdapter(adapter)

  await smokeTestLibquery(driverAdapter, '../../prisma/postgres/schema.prisma')
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
