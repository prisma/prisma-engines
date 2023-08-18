export { createPgConnector } from './pg'
export type { PrismaPgConfig } from './pg'

// import { Pool } from 'pg'

// const globalPool = new Pool({
//     connectionString: process.env["TEST_DATABASE_URL"],
//     ssl: {
//         rejectUnauthorized: false,
//     },
// })

// async function main() {
//     // const result = await globalPool.query("SELECT * FROM type_test")
//     const result = await globalPool.query(`DELETE FROM type_test`)
//     // const result = await globalPool.query(`select version()`)
//     console.log(result)
// }

// main()
