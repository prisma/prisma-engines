import {
  type DriverAdapter,
  type Query,
  type Result,
  type ResultSet,
} from "@prisma/driver-adapter-utils";

type Recordings = ReturnType<typeof createInMemoryRecordings>;

export function recording(adapter: DriverAdapter) {
  const recordings = createInMemoryRecordings();

  return {
    recorder: recorder(adapter, recordings),
    replayer: replayer(adapter, recordings),
  };
}

function recorder(adapter: DriverAdapter, recordings: Recordings) {
  return {
    provider: adapter.provider,
    startTransaction: () => {
      throw new Error("Not implemented");
    },
    getConnectionInfo: () => {
      return adapter.getConnectionInfo!();
    },
    queryRaw: async (params) => {
      const result = await adapter.queryRaw(params);
      recordings.addQueryResults(params, result);
      return result;
    },
    executeRaw: async (params) => {
      const result = await adapter.executeRaw(params);
      recordings.addCommandResults(params, result);
      return result;
    },
  };
}

function replayer(adapter: DriverAdapter, recordings: Recordings) {
  return {
    provider: adapter.provider,
    recordings: recordings,
    startTransaction: () => {
      throw new Error("Not implemented");
    },
    getConnectionInfo: () => {
      return adapter.getConnectionInfo!();
    },
    queryRaw: async (params) => {
      return recordings.getQueryResults(params);
    },
    executeRaw: async (params) => {
      return recordings.getCommandResults(params);
    },
  };
}

function createInMemoryRecordings() {
  const queryResults: Map<string, Result<ResultSet>> = new Map();
  const commandResults: Map<string, Result<number>> = new Map();

  const queryToKey = (params: Query) => {
    var sql = params.sql;
    params.args.forEach((arg: any, i) => {
      sql = sql.replace("$" + (i + 1), arg.toString());
    });
    return sql;
  };

  return {
    addQueryResults: (params: Query, result: Result<ResultSet>) => {
      const key = queryToKey(params);
      queryResults.set(key, result);
    },

    getQueryResults: (params: Query) => {
      const key = queryToKey(params);

      if (!queryResults.has(key)) {
        throw new Error(`Query not recorded: ${key}`);
      }

      return queryResults.get(key)!;
    },

    addCommandResults: (params: Query, result: Result<number>) => {
      const key = queryToKey(params);
      commandResults.set(key, result);
    },

    getCommandResults: (params: Query) => {
      const key = queryToKey(params);
      
      if (!commandResults.has(key)) {
        throw new Error(`Command not recorded: ${key}`);
      }

      return commandResults.get(key)!;
    },
  };
}
