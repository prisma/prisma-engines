import type { D1Database, D1PreparedStatement, D1Result } from '@cloudflare/workers-types'
import type { Queryable } from '@prisma/driver-adapter-utils'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const __dirname = path.dirname(fileURLToPath(import.meta.url))

export function copyPathName({ fromURL, toURL }: { fromURL: string; toURL: string }) {
  const toObj = new URL(toURL)
  toObj.pathname = new URL(fromURL).pathname

  return toObj.toString()
}

export function normaliseProvider(provider: Queryable['provider']) {
  return provider === 'postgres' ? 'postgresql' : provider
}

export function postgresSchemaName(url: string) {
  return new URL(url).searchParams.get('schema') ?? undefined
}

type PostgresOptions = {
  connectionString: string
  options?: string
}

export function postgresOptions(url: string): PostgresOptions {
  const args: PostgresOptions = { connectionString: url }

  const schemaName = postgresSchemaName(url)

  if (schemaName != null) {
    args.options = `--search_path="${schemaName}"`
  }

  return args
}

// Utility to avoid the `D1_ERROR: No SQL statements detected` error when running
// `D1_DATABASE.batch` with an empty array of statements.
export async function runBatch<T = unknown>(
  D1_DATABASE: D1Database,
  statements: D1PreparedStatement[],
): Promise<D1Result<T>[]> {
  if (statements.length === 0) {
    return []
  }

  return D1_DATABASE.batch(statements)
}

// conditional debug logging based on LOG_LEVEL env var
export const debug = (() => {
  if ((process.env.LOG_LEVEL ?? '').toLowerCase() != 'debug') {
    return (...args: any[]) => {}
  }

  return (...args: any[]) => {
    console.error('[nodejs] DEBUG:', ...args)
  }
})()

// error logger
export const err = (...args: any[]) => console.error('[nodejs] ERROR:', ...args)
