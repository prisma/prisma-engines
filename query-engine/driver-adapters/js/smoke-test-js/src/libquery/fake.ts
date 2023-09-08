import { PrismaFake } from '@jkomyno/prisma-adapter-fake'
import { bindAdapter } from '@jkomyno/prisma-driver-adapter-utils'
import { smokeTestLibquery } from './libquery'

async function main() {
  const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  // const pool = new pg.Pool({ connectionString })
  const adapter = new PrismaFake(undefined)
  const driverAdapter = bindAdapter(adapter)

  await smokeTestLibquery(driverAdapter, '../../prisma/postgres/schema.prisma')
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
