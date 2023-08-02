import { ColumnTypeEnum } from './const'

export type ColumnType = typeof ColumnTypeEnum[keyof typeof ColumnTypeEnum]

export interface ResultSet {
  columnTypes: Array<ColumnType>
  columnNames: Array<string>
  rows: Array<Array<any>>
}

export interface Query {
  sql: string
  args: Array<any>
}

export type Connector = {
  readonly flavor: 'mysql' | 'postgres',

  queryRaw: (params: Query) => Promise<ResultSet>
  executeRaw: (params: Query) => Promise<number>
  version: () => Promise<string | undefined>
  isHealthy: () => boolean
}

export type Closeable = {
  close: () => Promise<void>
}

export type ConnectorConfig
  = {
    host: string,
    username: string,
    password: string,
    url: never
  } | {
    url: string,
  }
