export const disabledTracingHelper = {
  isEnabled() {
    return false
  },
  getTraceParent() {
    // https://www.w3.org/TR/trace-context/#examples-of-http-traceparent-headers
    // If traceparent ends with -00 this trace will not be sampled
    // the query engine needs the `10` for the span and trace id otherwise it does not parse this
    return `00-10-10-00`
  },

  async createEngineSpan() {},

  getActiveContext() {
    return undefined
  },

  runInChildSpan<R>(options: string, callback: () => R): R {
    return callback()
  },
}

export type TracingHelper = typeof disabledTracingHelper
