import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const __dirname = path.dirname(fileURLToPath(import.meta.url))

export function copyPathName({ fromURL, toURL }: { fromURL: string, toURL: string }) {
  const toObj = new URL(fromURL)
  toObj.pathname = new URL(toURL).pathname

  return toObj.toString()
}

export function postgresSchemaName(url: string) {
  return new URL(url).searchParams.get('schema') ?? undefined
}

type PostgresOptions = {
  connectionString: string, options?: string
}

export function postgres_options(url: string): PostgresOptions {
  let args: PostgresOptions = { connectionString: url }
  
  const schemaName = postgresSchemaName(url)
  
  if (schemaName != null) {
      args.options = `--search_path="${schemaName}"`
  }

  return args
}
