import { createPlanetScaleConnector } from '@jkomyno/prisma-planetscale-js-connector'
import { smokeTest } from './test.js' 

async function planetscale() {
  const connectionString = `${process.env.JS_PLANETSCALE_DATABASE_URL as string}`

  const db = createPlanetScaleConnector({
    url: connectionString,
  })

  await smokeTest(db, '../prisma/mysql-planetscale/schema.prisma')
}

planetscale().catch((e) => {
  console.error(e)
  process.exit(1)
})
