import { webcrypto } from "node:crypto";
import * as qe from "./qe";

import pgDriver from "pg";
import * as prismaPg from "@prisma/adapter-pg";
import { DriverAdapter } from "@prisma/driver-adapter-utils";

import { recording } from "./recording";
import prismaQueries from "../bench/queries.json";

import { run, bench, group, baseline } from "mitata";

(global as any).crypto = webcrypto;

async function main(): Promise<void> {
  // read the prisma schema from stdin
  const prismaSchema = await new Promise<string>((resolve, reject) => {
    let data = "";
    process.stdin.on("data", (chunk) => {
      data += chunk;
    });
    process.stdin.on("end", () => {
      resolve(data);
    });
    process.stdin.on("error", reject);
  });

  const url = process.env.DATABASE_URL;
  if (url == null) {
    throw new Error("DATABASE_URL is not defined");
  }
  const pg = await pgAdapter(url);
  const { recorder, replayer } = recording(pg);

  await recordQueries(recorder, prismaSchema, prismaQueries);
  await benchMarkQueries(replayer, prismaSchema, prismaQueries);
}

async function recordQueries(
  adapter: DriverAdapter,
  prismaSchema: string,
  prismaQueries: any
): Promise<void> {
  const qe = await initQe(adapter, prismaSchema);
  await qe.connect("");

  for (const prismaQuery of prismaQueries) {
    const { description, query } = prismaQuery;
    console.error("Recording query: " + description);
    await qe.query(JSON.stringify(query), "", undefined);
  }
}

async function benchMarkQueries(
  adapter: DriverAdapter,
  prismaSchema: string,
  prismaQueries: any
) {
  const napi = await initQe(adapter, prismaSchema, "Napi");
  napi.connect("");
  const wasm = await initQe(adapter, prismaSchema, "Wasm");
  wasm.connect("");

  try {
    for (const prismaQuery of prismaQueries) {
      const { description, query } = prismaQuery;

      group(description, () => {
        bench("Node API", () =>
          napi.query(JSON.stringify(query), "", undefined)
        );
        bench("Web Assembly", () =>
          wasm.query(JSON.stringify(query), "", undefined)
        );
      });
    }

    await run({
      colors: true,
      collect: true,
    });
  } finally {
    napi.disconnect("");
    wasm.disconnect("");
  }
}

// conditional debug logging based on LOG_LEVEL env var
const debug = (() => {
  if ((process.env.LOG_LEVEL ?? "").toLowerCase() != "debug") {
    return (...args: any[]) => {};
  }

  return (...args: any[]) => {
    console.error("[nodejs] DEBUG:", ...args);
  };
})();

async function pgAdapter(url: string): Promise<DriverAdapter> {
  const schemaName = new URL(url).searchParams.get("schema") ?? undefined;
  let args: any = { connectionString: url };
  if (schemaName != null) {
    args.options = `--search_path="${schemaName}"`;
  }
  const pool = new pgDriver.Pool(args);

  return new prismaPg.PrismaPg(pool, {
    schema: schemaName,
  });
}

async function initQe(
  adapter: DriverAdapter,
  prismaSchema: string,
  engineType: "Wasm" | "Napi" = "Napi"
): Promise<qe.QueryEngine> {
  return await qe.initQueryEngine(
    engineType,
    adapter,
    prismaSchema,
    (...args) => {},
    debug
  );
}

const err = (...args: any[]) => console.error("[nodejs] ERROR:", ...args);

main().catch(err);
