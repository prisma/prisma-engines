import {
  type DriverAdapter,
  type Query,
  type Result,
  type ResultSet,
  type Transaction,
  TransactionOptions,
  ok,
} from "@prisma/driver-adapter-utils";

export const recording = (adapter: DriverAdapter) => {
  const recordings = new InMemoryRecordings();

  return {
    recorder: new Recorder(adapter, recordings),
    replayer: new Replayer(adapter, recordings),
  };
};

export interface Recordings {
  addQueryResults(params: Query, result: Result<ResultSet>);
  addCommandResults(params: Query, result: Result<number>);
  getQueryResults(params: Query): Result<ResultSet>;
  getCommandResults(params: Query): Result<number>;
}

export class InMemoryRecordings implements Recordings {
  readonly queryResults: Record<string, Result<ResultSet>> = {};
  readonly commandResults: Record<string, Result<number>> = {};

  addQueryResults(params: Query, result: Result<ResultSet>) {
    const key = this.queryToKey(params);
    if (key in this.queryResults) {
      throw new Error(`Query already recorded: ${key}`);
    }
    this.queryResults[key] = result;
  }

  getQueryResults(params: Query): Result<ResultSet> {
    const key = this.queryToKey(params);
    if (!(key in this.queryResults)) {
      throw new Error(`Query not recorded: ${key}`);
    }
    return this.queryResults[key];
  }

  addCommandResults(params: Query, result: Result<number>) {
    const key = this.queryToKey(params);
    if (key in this.commandResults) {
      throw new Error(`Command already recorded: ${key}`);
    }
    this.commandResults[key] = result;
  }

  getCommandResults(params: Query): Result<number> {
    const key = this.queryToKey(params);
    if (!(key in this.commandResults)) {
      throw new Error(`Command not recorded: ${key}`);
    }
    return this.commandResults[key];
  }

  protected queryToKey(query: Query): string {
    return JSON.stringify(query);
  }
}

export class Recorder implements DriverAdapter {
  provider: "mysql" | "postgres" | "sqlite";
  recordings: Recordings;
  adapter: DriverAdapter;

  constructor(adapter: DriverAdapter, recordings: Recordings) {
    this.adapter = adapter;
    this.provider = adapter.provider;
    this.recordings = recordings;
  }

  startTransaction(): Promise<Result<Transaction>> {
    throw new Error(
      "Not implemented. We didn't need transactions for our use case until now."
    );
  }

  get getConnectionInfo() {
    if (this.adapter && typeof this.adapter.getConnectionInfo === "function") {
      return () => this.adapter.getConnectionInfo!();
    }
    return undefined;
  }

  queryRaw(params: Query): Promise<Result<ResultSet>> {
    return new Promise((resolve, reject) => {
      this.adapter
        .queryRaw(params)
        .then((result) => {
          this.recordings.addQueryResults(params, result);
          resolve(result);
        })
        .catch((error) => {
          reject(error);
        });
    });
  }

  executeRaw(params: Query): Promise<Result<number>> {
    return new Promise((resolve, reject) => {
      this.adapter
        .executeRaw(params)
        .then((result) => {
          this.recordings.addCommandResults(params, result);
          resolve(result);
        })
        .catch((error) => {
          reject(error);
        });
    });
  }
}

export class Replayer implements DriverAdapter {
  provider: "mysql" | "postgres" | "sqlite";
  recordings: Recordings;
  adapter: DriverAdapter;

  constructor(adapter: DriverAdapter, recordings: Recordings) {
    this.adapter = adapter;
    this.provider = adapter.provider;
    this.recordings = recordings;
  }

  startTransaction(): Promise<Result<Transaction>> {
    throw new Error(
      "Not implemented. We didn't need transactions for our use case until now."
    );
  }

  get getConnectionInfo() {
    if (this.adapter && typeof this.adapter.getConnectionInfo === "function") {
      return () => this.adapter.getConnectionInfo!();
    }
    return undefined;
  }

  queryRaw(params: Query): Promise<Result<ResultSet>> {
    return new Promise((resolve, reject) => {
      try {
        const result = this.recordings.getQueryResults(params);
        resolve(result);
      } catch (error) {
        reject(error);
      }
    });
  }

  executeRaw(params: Query): Promise<Result<number>> {
    return new Promise((resolve, reject) => {
      try {
        const result = this.recordings.getCommandResults(params);
        resolve(result);
      } catch (error) {
        reject(error);
      }
    });
  }
}
