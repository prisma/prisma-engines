import { createNeonConnector } from '@jkomyno/prisma-neon-js-connector'
import { smokeTestClient } from './client'

async function neon() {
  const connectionString = `${process.env.DATABASE_URL as string}`

  const jsConnector = createNeonConnector({
    url: connectionString,
  })

  await smokeTestClient(jsConnector)
}

neon().catch((e) => {
  console.error(e)
  process.exit(1)
})
