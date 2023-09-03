import { createXataConnector } from '@jkomyno/prisma-xata-js-connector'
import { smokeTest } from './test'
import { getXataClient } from "./xata_gen";

async function xata() {
  // const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  const db = createXataConnector({
    // url: connectionString,
    xata: getXataClient
  })

  await smokeTest(db, '../prisma/postgres/schema.prisma')
}

xata().catch((e) => {
  console.error(e)
  process.exit(1)
})
