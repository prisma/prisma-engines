import { createPlanetScaleConnector } from '@jkomyno/prisma-planetscale-js-connector'
import { describe } from 'node:test'
import { smokeTestClient } from './client'

describe('planetscale with @prisma/client', async () => {
  const connectionString = `${process.env.JS_PLANETSCALE_DATABASE_URL as string}`

  const jsConnector = createPlanetScaleConnector({
    url: connectionString,
  })

  smokeTestClient(jsConnector)
})
