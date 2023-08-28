import { connect } from '@planetscale/database'
import { bindConnector } from '@jkomyno/prisma-js-connector-utils'
import { PlanetScaleAdapter } from '@jkomyno/prisma-planetscale-driver-adapter'
import { smokeTest } from './test' 

async function planetscale() {
  const connectionString = `${process.env.JS_PLANETSCALE_DATABASE_URL as string}`

  const planetscale = connect({
    url: connectionString,
  })

  const adapter = new PlanetScaleAdapter(planetscale)
  
  const db = bindConnector(adapter)
  await smokeTest(db, '../prisma/mysql-planetscale/schema.prisma')
}

planetscale().catch((e) => {
  console.error(e)
  process.exit(1)
})
