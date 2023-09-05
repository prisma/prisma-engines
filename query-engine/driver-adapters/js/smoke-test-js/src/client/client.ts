import { describe, it } from 'node:test'
import assert from 'node:assert'
import { PrismaClient } from '@prisma/client'
import type { DriverAdapter } from '@jkomyno/prisma-adapter-utils'

export async function smokeTestClient(connector: DriverAdapter) {
  const provider = connector.flavour

  const log = [
    {
      emit: 'event',
      level: 'query',
    } as const,
  ]

  for (const jsConnector of [connector, undefined]) {
    const isUsingJsConnector = jsConnector !== undefined
    describe(isUsingJsConnector ? `using JS Connectors` : `using Rust drivers`, () => {
      it('batch queries', async () => {
        const prisma = new PrismaClient({
          // @ts-ignore
          jsConnector,
          log,
        })
    
        const queries: string[] = []
        prisma.$on('query', ({ query }) => queries.push(query))

        await prisma.$transaction([
          prisma.$queryRawUnsafe('SELECT 1'),
          prisma.$queryRawUnsafe('SELECT 2'),
          prisma.$queryRawUnsafe('SELECT 3'),
        ])

        const defaultExpectedQueries = [
          'BEGIN',
          'SELECT 1',
          'SELECT 2',
          'SELECT 3',
          'COMMIT',
        ]

        const jsConnectorExpectedQueries = [
          '-- Implicit "BEGIN" query via underlying driver',
          'SELECT 1',
          'SELECT 2',
          'SELECT 3',
          '-- Implicit "COMMIT" query via underlying driver',
        ]

        const postgresExpectedQueries = [
          'BEGIN',
          'DEALLOCATE ALL',
          'SELECT 1',
          'SELECT 2',
          'SELECT 3',
          'COMMIT',
        ]

        if (['mysql'].includes(provider)) {
          if (isUsingJsConnector) {
            assert.deepEqual(queries, jsConnectorExpectedQueries)
          } else {
            assert.deepEqual(queries, defaultExpectedQueries)
          }
        } else if (['postgres'].includes(provider)) {
          if (isUsingJsConnector) {
            assert.deepEqual(queries, defaultExpectedQueries)
          } else {
            assert.deepEqual(queries, postgresExpectedQueries)
          }
        }
      })
    
      it('applies isolation level when using batch $transaction', async () => {
        const prisma = new PrismaClient({
          // @ts-ignore
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
