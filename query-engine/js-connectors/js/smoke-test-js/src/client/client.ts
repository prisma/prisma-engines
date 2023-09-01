import { Prisma, PrismaClient } from '@prisma/client'
import superjson from 'superjson'
import { ErrorCapturingConnector } from '@jkomyno/prisma-js-connector-utils'

export async function smokeTestClient(jsConnector: ErrorCapturingConnector) {
  const prisma = new PrismaClient({
    jsConnector,
    log: [
      {
        emit: 'event',
        level: 'query',
      },
    ],
  })

  prisma.$on('query', (e: Prisma.QueryEvent) => {
    const { json } = superjson.serialize(e)
    
    // @ts-ignore
    delete json['timestamp']
    // @ts-ignore
    delete json['duration']

    console.log('[nodejs] $on("query")', json)
  })

  prisma.$use(async (params, next) => {
    await Promise.resolve()
    return next(params)
  })

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

  await prisma.$transaction([
    prisma.child.findMany(),
    prisma.child.count(),
  ], {
    isolationLevel: 'ReadCommitted',
  })
}
