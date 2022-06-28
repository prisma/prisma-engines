'use strict'
const { QueryEngine } = require('./target/release/libquery_engine.dylib.node')

;(async () => {
  for (let i = 0; i < 50; i++) {
    const start = performance.now()
    for (let j = 0; j < 1000; j++) {
      const qe = new QueryEngine({
        datamodel: `
        datasource db {
          provider = "sqlite"
          url      = "file:./dev.db"
        }
        
        model User {
          id    String @id @default(uuid())
          email String @unique
        }
        `,
        logLevel: 'error',
        configDir: '.'
      }, () => {})
      qe.dropLogger()
    }
    console.log(`batch ${i} took ${performance.now() - start} ms`)
  }
})()