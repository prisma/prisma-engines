import { createPgConnector } from '@jkomyno/prisma-pg-js-connector'
import { describe } from 'node:test'
import { smokeTestClient } from './client'

describe('pg with @prisma/client', async () => {
  const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  const jsConnector = createPgConnector({
    url: connectionString,
  })

  smokeTestClient(jsConnector)
})
