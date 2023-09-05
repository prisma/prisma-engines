import { createNeonConnector } from '@jkomyno/prisma-neon-js-connector'
import { describe } from 'node:test'
import { smokeTestClient } from './client'

describe('neon with @prisma/client', async () => {
  const connectionString = `${process.env.JS_NEON_DATABASE_URL as string}`

  const jsConnector = createNeonConnector({
    url: connectionString,
  })

  smokeTestClient(jsConnector)
})
