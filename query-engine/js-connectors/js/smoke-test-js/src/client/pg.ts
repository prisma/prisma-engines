import { createPgConnector } from '@jkomyno/prisma-pg-js-connector'
import { smokeTestClient } from './client'

async function pg() {
  const connectionString = `${process.env.DATABASE_URL as string}`

  const jsConnector = createPgConnector({
    url: connectionString,
  })

  await smokeTestClient(jsConnector)
}

pg().catch((e) => {
  console.error(e)
  process.exit(1)
})
