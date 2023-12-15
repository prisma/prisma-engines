/**
 * Run with: `node --experimental-wasm-modules ./example.js`
 * on Node.js 18+.
 */
import { webcrypto } from "node:crypto";
import * as qe from "./qe";

import pgDriver from "pg";
import * as prismaPg from "@prisma/adapter-pg";
import { DriverAdapter } from "@prisma/driver-adapter-utils";

import { recording } from "./recording";
import prismaQueries from "../bench/queries.json";

import { run, bench, group, baseline } from "mitata";

import { QueryEngine as WasmBaseline } from "query-engine-wasm-baseline";
import { QueryEngine as WasmLatest } from "query-engine-wasm-latest";

(global as any).crypto = webcrypto;

async function main(): Promise<void> {
  // read the prisma schema from stdin
  const datamodel = await new Promise<string>((resolve, reject) => {
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

  await recordQueries(recorder, datamodel, prismaQueries);
  await benchMarkQueries(replayer, datamodel, prismaQueries);
}

async function recordQueries(
  adapter: DriverAdapter,
  datamodel: string,
  prismaQueries: any
): Promise<void> {
  const qe = await initQeNapiCurrent(adapter, datamodel);
  await qe.connect("");

  for (const prismaQuery of prismaQueries) {
    const { description, query } = prismaQuery;
    console.error("Recording query: " + description);
    await qe.query(JSON.stringify(query), "", undefined);
  }
}

async function benchMarkQueries(
  adapter: DriverAdapter,
  datamodel: string,
  prismaQueries: any
) {
  const napi = await initQeNapiCurrent(adapter, datamodel);
  napi.connect("");
  const wasmCurrent = await initQeWasmCurrent(adapter, datamodel);
  wasmCurrent.connect("");
  const wasmBaseline = await initQeWasmBaseLine(adapter, datamodel);
  wasmBaseline.connect("");
  const wasmLatest = await initQeWasmLatest(adapter, datamodel);
  wasmLatest.connect("");

  try {
    for (const prismaQuery of prismaQueries) {
      const { description, query } = prismaQuery;
      const jsonQuery = JSON.stringify(query);

      group(description, () => {
        bench("Web Assembly: Baseline", () =>
          wasmBaseline.query(jsonQuery, "", undefined)
        );

        bench("Web Assembly: Latest", () =>
          wasmLatest.query(jsonQuery, "", undefined)
        );

        baseline("Web Assembly: Current", () =>
          wasmCurrent.query(jsonQuery, "", undefined)
        );

        bench("Node API: Current", () => napi.query(jsonQuery, "", undefined));
      });
    }

    await run({
      colors: false,
      collect: true,
    });
  } finally {
    napi.disconnect("");
    wasmCurrent.disconnect("");
    wasmBaseline.disconnect("");
    wasmLatest.disconnect("");
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

async function initQeNapiCurrent(
  adapter: DriverAdapter,
  datamodel: string
): Promise<qe.QueryEngine> {
  return await qe.initQueryEngine(
    "Napi",
    adapter,
    datamodel,
    (...args) => {},
    debug
  );
}

async function initQeWasmCurrent(
  adapter: DriverAdapter,
  datamodel: string
): Promise<qe.QueryEngine> {
  return await qe.initQueryEngine(
    "Wasm",
    adapter,
    datamodel,
    (...args) => {},
    debug
  );
}

async function initQeWasmLatest(
  adapter: DriverAdapter,
  datamodel: string
): Promise<qe.QueryEngine> {
  return new WasmLatest(qe.queryEngineOptions(datamodel), () => {}, adapter);
}

function initQeWasmBaseLine(
  adapter: DriverAdapter,
  datamodel: string
): qe.QueryEngine {
  return new WasmBaseline(qe.queryEngineOptions(datamodel), () => {}, adapter);
}

const err = (...args: any[]) => console.error("[nodejs] ERROR:", ...args);

main().catch(err);
