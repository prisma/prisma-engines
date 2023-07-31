import { createNeonConnector } from './connector/neon.js'
import { smokeTest } from './test.js' 

async function neon() {
  const connectionString = `${process.env.JS_NEON_DATABASE_URL as string}`

  const db = createNeonConnector({
    url: connectionString,
  })

  await smokeTest(db, '../prisma/postgres-neon/schema.prisma')
}

neon().catch((e) => {
  console.error(e)
  process.exit(1)
})
