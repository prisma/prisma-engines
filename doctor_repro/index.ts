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

    const start = Date.now();
    const last_seen = await prisma.user.findMany({ orderBy: { latest_purchase: 'desc' }, take: 5 })
    const names = last_seen?.map((u) => u.username || "Mysterious User");
    console.log("Latest buyers", names);

    const sum = await prisma.purchase.aggregate({ _sum: { price: true }, where: { users: { username: { in: names } } } })
    console.log("... They spent $", sum._sum, "in purchases");
    console.log(`Debug: queries took: ${Date.now() - start} ms`);
}

void main().catch(async (e) => {
    console.log("Error propagated to main", e)

})


