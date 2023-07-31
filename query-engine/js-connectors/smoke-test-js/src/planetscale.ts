
import { setImmediate, setTimeout } from 'node:timers/promises'

import { binder } from './connector/util.js'
import { createPlanetScaleConnector } from './connector/planetscale.js'
import { initQueryEngine } from './util.js'
import type { QueryEngineInstance } from './engines/types/Library.js'

async function main() {
  const connectionString = `${process.env.JS_PLANETSCALE_DATABASE_URL as string}`

  /* Use `db` if you want to test the actual PlanetScale database */
  const db = createPlanetScaleConnector({
    url: connectionString,
  })

  // `binder` is required to preserve the `this` context to the group of functions passed to libquery.
  const conn = binder(db)

  // wait for the database pool to be initialized
  await setImmediate(0)

  const engine = initQueryEngine(conn)

  console.log('[nodejs] connecting...')
  await engine.connect('trace')
  console.log('[nodejs] connected')

  console.log('[nodejs] isHealthy', await conn.isHealthy())

  await testFindManyTypeTest(engine)
  await testCreateAndDeleteChildParent(engine)

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
  await conn.close()
  console.log('[nodejs] closed database connection')

  process.exit(0)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})


// Smoke test for PlanetScale that ensures we're able to decode every common data type.
// Repurposed from: `query_engine_tests::writes::top_level_mutations::delete_many_relations::delete_many_rels::p1_c1`
async function testFindManyTypeTest(engine: QueryEngineInstance) {
  const resultSet = await engine.query(`
    {
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
    }
  `, 'trace', undefined)
  console.log('[nodejs] findMany resultSet', JSON.stringify(JSON.parse(resultSet), null, 2))

  return resultSet
}

/**
 * The following code creates several transactions (each time a deletion is requested).
 * In particular:
 * - 1st transaction: Delete all child and parent records
 * - Create a parent with some new children, within a transaction
 */
async function testCreateAndDeleteChildParent(engine: QueryEngineInstance) {
  /* Delete all child and parent records */

  // Queries: [
  //   'SELECT `cf-users`.`Child`.`id` FROM `cf-users`.`Child` WHERE 1=1',
  //   'SELECT `cf-users`.`Child`.`id` FROM `cf-users`.`Child` WHERE 1=1',
  //   'DELETE FROM `cf-users`.`Child` WHERE (`cf-users`.`Child`.`id` IN (?) AND 1=1)'
  // ]
  await engine.query(`
    {
      "modelName": "Child",
      "action": "deleteMany",
      "query": {
        "arguments": {
          "where": {}
        },
        "selection": {
          "count": true
        }
      }
    }
  `, 'trace', undefined)

  // Queries: [
  //   'SELECT `cf-users`.`Parent`.`id` FROM `cf-users`.`Parent` WHERE 1=1',
  //   'SELECT `cf-users`.`Parent`.`id` FROM `cf-users`.`Parent` WHERE 1=1',
  //   'DELETE FROM `cf-users`.`Parent` WHERE (`cf-users`.`Parent`.`id` IN (?) AND 1=1)'
  // ]
  await engine.query(`
    {
      "modelName": "Parent",
      "action": "deleteMany",
      "query": {
        "arguments": {
          "where": {}
        },
        "selection": {
          "count": true
        }
      }
    }
  `, 'trace', undefined)

  /* Create a parent with some new children, within a transaction */

  // Queries: [
  //   'INSERT INTO `cf-users`.`Parent` (`p`,`p_1`,`p_2`,`id`) VALUES (?,?,?,?)',
  //   'INSERT INTO `cf-users`.`Child` (`c`,`c_1`,`c_2`,`parentId`,`id`) VALUES (?,?,?,?,?)',
  //   'SELECT `cf-users`.`Parent`.`id`, `cf-users`.`Parent`.`p` FROM `cf-users`.`Parent` WHERE `cf-users`.`Parent`.`id` = ? LIMIT ? OFFSET ?',
  //   'SELECT `cf-users`.`Child`.`id`, `cf-users`.`Child`.`c`, `cf-users`.`Child`.`parentId` FROM `cf-users`.`Child` WHERE `cf-users`.`Child`.`parentId` IN (?)'
  // ]
  await engine.query(`
    {
      "modelName": "Parent",
      "action": "createOne",
      "query": {
        "arguments": {
          "data": {
            "p": "p1",
            "p_1": "1",
            "p_2": "2",
            "childOpt": {
              "create": {
                "c": "c1",
                "c_1": "foo",
                "c_2": "bar"
              }
            }
          }
        },
        "selection": {
          "p": true,
          "childOpt": {
            "arguments": null,
            "selection": {
              "c": true
            }
          }
        }
      }
    }
  `, 'trace', undefined)

  /* Delete the parent */

  // Queries: [
  //   'SELECT `cf-users`.`Parent`.`id` FROM `cf-users`.`Parent` WHERE `cf-users`.`Parent`.`p` = ?',
  //   'SELECT `cf-users`.`Child`.`id`, `cf-users`.`Child`.`parentId` FROM `cf-users`.`Child` WHERE (1=1 AND `cf-users`.`Child`.`parentId` IN (?))',
  //   'UPDATE `cf-users`.`Child` SET `parentId` = ? WHERE (`cf-users`.`Child`.`id` IN (?) AND 1=1)',
  //   'SELECT `cf-users`.`Parent`.`id` FROM `cf-users`.`Parent` WHERE `cf-users`.`Parent`.`p` = ?',
  //   'DELETE FROM `cf-users`.`Parent` WHERE (`cf-users`.`Parent`.`id` IN (?) AND `cf-users`.`Parent`.`p` = ?)'
  // ]
  const resultDeleteMany = await engine.query(`
    {
      "modelName": "Parent",
      "action": "deleteMany",
      "query": {
        "arguments": {
          "where": {
            "p": "p1"
          }
        },
        "selection": {
          "count": true
        }
      }
    }
  `, 'trace', undefined)
  console.log('[js] resultDeleteMany', JSON.stringify(JSON.parse(resultDeleteMany), null, 2))
}
