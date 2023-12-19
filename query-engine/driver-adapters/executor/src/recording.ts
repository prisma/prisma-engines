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

  const queryToKey = (query) => JSON.stringify(query);

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
