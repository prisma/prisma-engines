import { PrismaFake } from '@jkomyno/prisma-adapter-fake'
import { bindAdapter } from '@jkomyno/prisma-driver-adapter-utils'
import { smokeTestLibquery } from './libquery'

async function main() {
  const connectionString = `${process.env.JS_PG_DATABASE_URL as string}`

  // const pool = new pg.Pool({ connectionString })
  const driver = { 
    query: (sql, params) => { 
      console.log("& query()", sql, params)
      if(sql == `INSERT INTO "public"."Product" ("id","properties","properties_null") VALUES ($1,$2,$3) RETURNING "public"."Product"."id", "public"."Product"."properties"`)
        return { fields: [{name: "id", dataTypeID: 25}, {name: "properties", dataTypeID: 3802}, {name: "properties_null", dataTypeID: 3802}], rows: [{id: params[0], properties: params[1], properties_null: params[2]}] }
      if(sql == `SELECT "public"."Product"."id", "public"."Product"."properties", "public"."Product"."properties_null" FROM "public"."Product" WHERE 1=1 OFFSET $1`)
        return { fields: [{name: "id", dataTypeID: 25}, {name: "properties", dataTypeID: 3802}, {name: "properties_null", dataTypeID: 3802}], rows: [{id: "clm9yv28n000008l91y6r2d6v", properties: {foo:"bar"}, properties_null: null}] }
      if(sql == `INSERT INTO "public"."type_test_2" ("id","datetime_column") VALUES ($1,$2) RETURNING "public"."type_test_2"."id", "public"."type_test_2"."datetime_column", "public"."type_test_2"."datetime_column_null"`)
        return { fields: [{name: "id", dataTypeID: 25}, {name: "datetime_column", dataTypeID: 1114}, {name: "datetime_column_null", dataTypeID: 1114}], rows: [{id: "clm9yv28n000008l91y6r2d6v", datetime_column: '2023-09-08 12:34:56', datetime_column_null: null}] }
      if(sql == `SELECT "public"."type_test_2"."id", "public"."type_test_2"."datetime_column", "public"."type_test_2"."datetime_column_null" FROM "public"."type_test_2" WHERE 1=1 OFFSET $1`)
        return { fields: [{name: "id", dataTypeID: 25}, {name: "datetime_column", dataTypeID: 1114}, {name: "datetime_column_null", dataTypeID: 1114}], rows: [{id: "clm9yv28n000008l91y6r2d6v", datetime_column: '2023-09-08 12:34:56', datetime_column_null: null}] }
      if(sql == `SELECT "public"."type_test"."id", "public"."type_test"."smallint_column", "public"."type_test"."int_column", "public"."type_test"."bigint_column", "public"."type_test"."float_column", "public"."type_test"."double_column", "public"."type_test"."decimal_column", "public"."type_test"."boolean_column", "public"."type_test"."char_column", "public"."type_test"."varchar_column", "public"."type_test"."text_column", "public"."type_test"."date_column", "public"."type_test"."time_column", "public"."type_test"."datetime_column", "public"."type_test"."timestamp_column", "public"."type_test"."json_column", "public"."type_test"."enum_column" FROM "public"."type_test" WHERE 1=1 OFFSET $1`)
        return { fields: [], rows: [] } // TODO Actually return useful data instead of pretending we found nothing - btw why does the code not fail?
      if(sql == `INSERT INTO "public"."authors" ("firstName","lastName","age") VALUES ($1,$2,$3) RETURNING "public"."authors"."id", "public"."authors"."firstName", "public"."authors"."lastName"`) //  [ 'Firstname from autoincrement', 'Lastname from autoincrement', 99 ]
        return { fields: [{name: "id", dataTypeID: 20}, {name: "firstName", dataTypeID: 25}, {name: "lastName", dataTypeID: 25}], rows: [{ id: "1", firstName: params[0], lastName: params[1] }] }

      // resultDeleteMany
      if(sql == `SELECT "public"."authors"."id", "public"."authors"."firstName", "public"."authors"."lastName", "public"."authors"."age" FROM "public"."authors" WHERE 1=1 OFFSET $1 [ 0 ]`)
        return { fields: [{name: "id", dataTypeID: 20}, {name: "firstName", dataTypeID: 25}, {name: "lastName", dataTypeID: 25}, {name: "age", dataTypeId: 20}], rows: [{ id: "1", firstName: "first", lastName: "last", age: 50 }] }

    },
    connect: () => { 
      console.log("& connect()")
      return {
        query: (sql, params) => {
          console.log("transaction query()", sql, params)
          if(sql == `BEGIN` || sql == `ROLLBACK`)
            return -1
          if(sql == `SELECT "public"."Product"."id" FROM "public"."Product" WHERE 1=1 OFFSET $1`)
            return "foo"
          
        },
        release: () => { 
          console.log("& release()")
          return -1
        }
      }
    }
  }
  const adapter = new PrismaFake(driver)
  const driverAdapter = bindAdapter(adapter)

  await smokeTestLibquery(driverAdapter, '../../prisma/postgres/schema.prisma')
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
