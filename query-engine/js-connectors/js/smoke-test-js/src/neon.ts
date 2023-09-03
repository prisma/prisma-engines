import { createNeonConnector } from '@jkomyno/prisma-neon-js-connector'
import { smokeTest } from './test'

async function neon() {
  const connectionString = `${process.env.JS_NEON_DATABASE_URL as string}`

  const db = createNeonConnector({
    url: connectionString,
    httpMode: false,
  })

  await smokeTest(db, '../prisma/postgres/schema.prisma')
}

neon().catch((e) => {
  console.error(e)
  process.exit(1)
})
