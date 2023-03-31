export class ConnectorProxy {
  constructor(queryable) {
    this.queryable = queryable
  }

  connect() {
    return queryable.connect()
  }

  disconnect() {
    return this.queryable.disconnect()
  }

  /// Execute the given query.
  query(query) {
    return this.queryable.query(query)
  }

  /// Execute a query given as SQL, interpolating the given parameters.
  query_raw(query, params) {
    return this.queryable.query_raw(query, params)
  }

  /// Returns false, if connection is considered to not be in a working state.
  is_healthy() {
    return queryable.is_healthy()
  }
}