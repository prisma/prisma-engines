import { describe, it } from 'node:test'
import assert from 'node:assert'
import { PrismaClient } from '@prisma/client'
import { ErrorCapturingConnector } from '@jkomyno/prisma-js-connector-utils'

export async function smokeTestClient(connector: ErrorCapturingConnector) {
  const provider = connector.flavour

  const log = [
    {
      emit: 'event',
      level: 'query',
    } as const,
  ]

  for (const jsConnector of [connector, undefined]) {
    describe(jsConnector ? `using JS Connectors` : `using Rust drivers`, () => {
      it('executes batch queries in the right order when using extensions + middleware', async () => {
        const prisma = new PrismaClient({
          jsConnector,
          log,
        })
    
        const queries: string[] = []
        prisma.$on('query', ({ query }) => queries.push(query))
    
        const xprisma = prisma.$extends({
          query: {
            async $queryRawUnsafe({ args, query }) {
              const [, result] = await prisma.$transaction([
                prisma.$queryRawUnsafe('SELECT 1'),
                query(args),
                prisma.$queryRawUnsafe('SELECT 3'),
              ])
              return result
            },
          },
        })
    
        await xprisma.$queryRawUnsafe('SELECT 2')
    
        assert.deepEqual(queries, [
          'BEGIN',
          'SELECT 1',
          'SELECT 2',
          'SELECT 3',
          'COMMIT',
        ])
      })
    
      it('applies isolation level when using batch $transaction', async () => {
        const prisma = new PrismaClient({
          jsConnector,
          log,
        })
    
        const queries: string[] = []
        prisma.$on('query', ({ query }) => queries.push(query))
    
        await prisma.$transaction([
          prisma.child.findMany(),
          prisma.child.count(),
        ], {
          isolationLevel: 'ReadCommitted',
        })
    
        if (['mysql'].includes(provider)) {
          assert.deepEqual(queries.slice(0, 2), [
            'SET TRANSACTION ISOLATION LEVEL READ COMMITTED',
            'BEGIN',
          ])
        } else if (['postgres'].includes(provider)) {
          assert.deepEqual(queries.slice(0, 2), [
            'BEGIN',
            'SET TRANSACTION ISOLATION LEVEL READ COMMITTED',
          ])
        }
    
        assert.deepEqual(queries.at(-1), 'COMMIT')
      })
    })
  }
}
