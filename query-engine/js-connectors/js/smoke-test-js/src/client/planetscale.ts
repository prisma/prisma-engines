import { createPlanetScaleConnector } from '@jkomyno/prisma-planetscale-js-connector'
import { smokeTestClient } from './client'

async function planetscale() {
  const connectionString = `${process.env.DATABASE_URL as string}`

  const jsConnector = createPlanetScaleConnector({
    url: connectionString,
  })

  await smokeTestClient(jsConnector)
}

planetscale().catch((e) => {
  console.error(e)
  process.exit(1)
})
