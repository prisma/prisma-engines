import { setImmediate } from 'node:timers/promises'
import type { Connector } from '@jkomyno/prisma-js-connector-utils'
import type { QueryEngineInstance } from './engines/types/Library'
import { initQueryEngine } from './util'

type Flavor = Connector['flavour']

export async function smokeTest(db: Connector, prismaSchemaRelativePath: string) {
  // wait for the database pool to be initialized
  await setImmediate(0)
  
  const engine = initQueryEngine(db, prismaSchemaRelativePath)

  console.log('[nodejs] connecting...')
  await engine.connect('trace')
  console.log('[nodejs] connected')

  // console.log('[nodejs] isHealthy', await conn.isHealthy())

  const test = new SmokeTest(engine, db.flavour)

  // await test.testFindManyTypeTest()
  await test.createAutoIncrement()
  // await test.testCreateAndDeleteChildParent()
  // await test.testTransaction()

  // Note: calling `engine.disconnect` won't actually close the database connection.
  console.log('[nodejs] disconnecting...')
  await engine.disconnect('trace')
  console.log('[nodejs] disconnected')

  await setImmediate(0)

  console.log('[nodejs] re-connecting...')
  await engine.connect('trace')
  console.log('[nodejs] re-connecting')

  await setImmediate(0)

  console.log('[nodejs] re-disconnecting...')
  await engine.disconnect('trace')
  console.log('[nodejs] re-disconnected')
}

class SmokeTest {
  constructor(private readonly engine: QueryEngineInstance, readonly flavour: Connector['flavour']) {}

  async testFindManyTypeTest() {
    await this.testFindManyTypeTestMySQL()
    await this.testFindManyTypeTestPostgres()
  }

  private async testFindManyTypeTestMySQL() {
    if (this.flavour !== 'mysql') {
      return
    }

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

  private async testFindManyTypeTestPostgres() {
    if (this.flavour !== 'postgres') {
      return
    }

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

  async createAutoIncrement() {
    await this.engine.query(`
      {
        "modelName": "Author",
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

    const author = await this.engine.query(`
      {
        "modelName": "Author",
        "action": "createOne",
        "query": {
          "arguments": {
            "data": {
              "firstName": "Firstname from autoincrement",
              "lastName": "Lastname from autoincrement",
              "age": 99
            }
          },
          "selection": {
            "id": true,
            "firstName": true,
            "lastName": true
          }
        }
      }
    `, 'trace', undefined)
    console.log('[nodejs] author', JSON.stringify(JSON.parse(author), null, 2))
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

  async testTransaction() {
    const startResponse = await this.engine.startTransaction(JSON.stringify({ isolation_level: 'Serializable', max_wait: 5000, timeout: 15000 }), 'trace')

    const tx_id = JSON.parse(startResponse).id

    console.log('[nodejs] transaction id', tx_id)
    await this.engine.query(`
    {
      "action": "findMany",
      "modelName": "Author",
      "query": {
        "selection": { "$scalars": true }
      }
    }
    `, 'trace', tx_id)

    const commitResponse = await this.engine.commitTransaction(tx_id, 'trace')
    console.log('[nodejs] commited', commitResponse)
  }
}
