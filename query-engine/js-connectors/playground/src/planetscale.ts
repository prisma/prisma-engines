
import { setImmediate, setTimeout } from 'node:timers/promises'

import { binder } from './driver/util.js'
import { createPlanetScaleDriver } from './driver/planetscale.js'
import { initQueryEngine } from './util.js'

async function main() {
  const connectionString = `${process.env.JS_PLANETSCALE_DATABASE_URL as string}`

  /* Use `db` if you want to test the actual PlanetScale database */
  const db = createPlanetScaleDriver({
    url: connectionString,
  })

  // `binder` is required to preserve the `this` context to the group of functions passed to libquery.
  const driver = binder(db)

  // wait for the database pool to be initialized
  await setImmediate(0)

  const engine = initQueryEngine(driver)

  console.log('[nodejs] connecting...')
  await engine.connect('trace')
  console.log('[nodejs] connected')

  console.log('[nodejs] isHealthy', await driver.isHealthy())

  // Smoke test for PlanetScale that ensures we're able to decode every common data type.
  const resultSet = await engine.query(`{
    "action": "findMany",
    "modelName": "type_test",
    "query": {
      "selection": {
        "tinyint_column": true,
        "smallint_column": true,
        "mediumint_column": true,
        "int_column": true,
        "bigint_column": true,
        "float_column": true,
        "double_column": true,
        "decimal_column": true,
        "boolean_column": true,
        "char_column": true,
        "varchar_column": true,
        "text_column": true,
        "date_column": true,
        "time_column": true,
        "datetime_column": true,
        "timestamp_column": true,
        "json_column": true,
        "enum_column": true,
        "binary_column": true,
        "varbinary_column": true,
        "blob_column": true
      }
    } 
    }`, 'trace', undefined)

  console.log('[nodejs] findMany resultSet', JSON.stringify(JSON.parse(resultSet), null, 2))

  // Note: calling `engine.disconnect` won't actually close the database connection.
  console.log('[nodejs] disconnecting...')
  await engine.disconnect('trace')
  console.log('[nodejs] disconnected')

  console.log('[nodejs] re-connecting...')
  await engine.connect('trace')
  console.log('[nodejs] re-connecting')

  await setTimeout(0)

  console.log('[nodejs] re-disconnecting...')
  await engine.disconnect('trace')
  console.log('[nodejs] re-disconnected')

  // Close the database connection. This is required to prevent the process from hanging.
  console.log('[nodejs] closing database connection...')
  await driver.close()
  console.log('[nodejs] closed database connection')

  process.exit(0)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
