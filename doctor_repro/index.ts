import { PrismaClient, Prisma } from '.prisma/client'

async function main() {
    const prisma = new PrismaClient({
        log: [
            {
                emit: 'stdout',
                level: 'error',
            },
            {
                emit: 'stdout',
                level: 'info',
            },
            {
                emit: 'stdout',
                level: 'query',
            },
        ],
        __internal: {
            engine: {
                endpoint: "http://127.0.0.1:57581",
            },
        },
    } as any);

    const tees = await prisma.user.findMany({ where: { username: { startsWith: "T" } } })
    console.log(tees);

    const first = await prisma.user.findFirst({ orderBy: { username: 'desc' } })
    console.log(first);
}

void main().catch(async (e) => {
    console.log("Error propagated to main", e)
})