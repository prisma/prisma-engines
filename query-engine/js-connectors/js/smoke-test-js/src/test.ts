import { setImmediate, setTimeout } from 'node:timers/promises'
import type { Connector, Closeable } from '@jkomyno/prisma-js-connector-utils'
import type { QueryEngineInstance } from './engines/types/Library'
import { initQueryEngine } from './util'

type Flavor = Connector['flavour']

export async function smokeTest(db: Connector & Closeable, prismaSchemaRelativePath: string) {
  // wait for the database pool to be initialized
  await setImmediate(0)
  
  const engine = initQueryEngine(db, prismaSchemaRelativePath)

  console.log('[nodejs] connecting...')
  await engine.connect('trace')
  console.log('[nodejs] connected')

  // console.log('[nodejs] isHealthy', await conn.isHealthy())

  const test = new SmokeTest(engine, db.flavour)

  await test.testFindManyTypeTest()
  await test.testCreateAndDeleteChildParent()

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
  await db.close()
  console.log('[nodejs] closed database connection')

  process.exit(0)
}

class SmokeTest {
  constructor(private readonly engine: QueryEngineInstance, readonly flavour: Connector['flavour']) {}

  async testFindManyTypeTest() {
    await this.testFindManyTypeTestMySQL()
    await this.testFindManyTypeTestPostgres()
  }

  @withFlavor({ only: ['mysql'] })
  private async testFindManyTypeTestMySQL() {
    const resultSet = await this.engine.query(`
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

  @withFlavor({ only: ['postgres'] })
  private async testFindManyTypeTestPostgres() {
    const resultSet = await this.engine.query(`
      {
        "action": "findMany",
        "modelName": "type_test",
        "query": {
          "selection": {
            "smallint_column": true,
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
            "enum_column": true
          }
        } 
      }
    `, 'trace', undefined)
    console.log('[nodejs] findMany resultSet', JSON.stringify(JSON.parse(resultSet), null, 2))
  
    return resultSet
  }

  async testCreateAndDeleteChildParent() {
    /* Delete all child and parent records */
  
    // Queries: [
    //   'SELECT `cf-users`.`Child`.`id` FROM `cf-users`.`Child` WHERE 1=1',
    //   'SELECT `cf-users`.`Child`.`id` FROM `cf-users`.`Child` WHERE 1=1',
    //   'DELETE FROM `cf-users`.`Child` WHERE (`cf-users`.`Child`.`id` IN (?) AND 1=1)'
    // ]
    await this.engine.query(`
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
    await this.engine.query(`
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
    await this.engine.query(`
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
    const resultDeleteMany = await this.engine.query(`
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
    console.log('[nodejs] resultDeleteMany', JSON.stringify(JSON.parse(resultDeleteMany), null, 2))
  }
}

type WithFlavorInput
  = { only: Array<Flavor>, exclude?: never }
  | { exclude: Array<Flavor>, only?: never }

function withFlavor({ only, exclude }: WithFlavorInput) {
  return function decorator(originalMethod: () => any, _ctx: ClassMethodDecoratorContext<SmokeTest, () => unknown>) {
    return function replacement(this: SmokeTest) {
      if ((exclude || []).includes(this.flavour)) {
        console.log(`[nodejs::exclude] Skipping test "${originalMethod.name}" with flavour: ${this.flavour}`)
        return
      }

      if ((only || []).length > 0 && !(only || []).includes(this.flavour)) {
        console.log(`[nodejs::only] Skipping test "${originalMethod.name}" with flavour: ${this.flavour}`)
        return
      }

      return originalMethod.call(this)
    }
  }
}
