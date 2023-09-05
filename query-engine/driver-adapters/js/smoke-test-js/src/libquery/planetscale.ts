import { createPlanetScaleConnector } from '@jkomyno/prisma-planetscale-js-connector'
import { smokeTestLibquery } from './libquery' 

async function planetscale() {
  const connectionString = `${process.env.JS_PLANETSCALE_DATABASE_URL as string}`

  const db = createPlanetScaleConnector({
    url: connectionString,
  })

  await smokeTestLibquery(db, '../../prisma/mysql/schema.prisma')
}

planetscale().catch((e) => {
  console.error(e)
  process.exit(1)
})
