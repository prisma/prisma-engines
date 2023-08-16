import type { Connector } from './types';

// *.bind(db) is required to preserve the `this` context.
// There are surely other ways than this to use class methods defined in JS within a
// driver context, but this is the most straightforward.
export const binder = (queryable: Connector): Connector => ({
  queryRaw: queryable.queryRaw.bind(queryable),
  executeRaw: queryable.executeRaw.bind(queryable),
  close: queryable.close.bind(queryable),
  flavour: queryable.flavour,
})
