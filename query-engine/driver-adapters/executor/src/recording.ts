import {
  type DriverAdapter,
  type Query,
  type Result,
  type ResultSet,
} from "@prisma/driver-adapter-utils";
import { RetryHandler } from "undici";

export function recording(adapter: DriverAdapter) {
  const recordings = createInMemoryRecordings();

  return {
    recorder: recorder(adapter, recordings),
    replayer: replayer(adapter, recordings),
  };
}

function recorder(adapter: DriverAdapter, recordings) {
  return {
    provider: adapter.provider,
    startTransaction: () => {
      throw new Error("Not implemented");
    },
    getConnectionInfo: () => {
      return adapter.getConnectionInfo!();
    },
    queryRaw: async (params) => {
      const result = adapter.queryRaw(params);
      recordings.addQueryResults(params, result);
      return result;
    },

    executeRaw: async (params) => {
      const result = adapter.executeRaw(params);
      recordings.addCommandResults(params, result);
      return result;
    },
  };
}

function replayer(adapter: DriverAdapter, recordings) {
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
  const queryResults = {};
  const commandResults = {};

  // Recording is currently only used in benchmarks. Before we used to serialize the whole query
  // (sql + args) but since bigints are not serialized by JSON.stringify, and we didn't really need
  // args for benchmarks, we just serialize the sql part.
  //
  // If this ever changes (we reuse query recording in tests) we need to make sure to serialize the
  // args as well.
  const queryToKey = (query) => JSON.stringify(query.sql);

  return {
    addQueryResults: (params, result) => {
      const key = queryToKey(params);
      queryResults[key] = result;
    },

    getQueryResults: (params) => {
      const key = queryToKey(params);
      if (!(key in queryResults)) {
        throw new Error(`Query not recorded: ${key}`);
      }
      return queryResults[key];
    },

    addCommandResults: (params, result) => {
      const key = queryToKey(params);
      commandResults[key] = result;
    },

    getCommandResults: (params) => {
      const key = queryToKey(params);
      if (!(key in commandResults)) {
        throw new Error(`Command not recorded: ${key}`);
      }
      return commandResults[key];
    },
  };
}
