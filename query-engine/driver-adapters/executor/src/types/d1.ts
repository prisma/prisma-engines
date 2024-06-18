import * as S from '@effect/schema/Schema'

const D1Table = S.union(
  S.struct({
    schema: S.union(S.literal('main'), S.string),
    name: S.string,
    type: S.literal('table', 'view', 'shadow', 'virtual'),
  }),
  S.struct({
    schema: S.literal('main'),
    name: S.literal('sqlite_sequence'),
    type: S.literal('table'),
  }),
  S.struct({
    schema: S.literal('main'),
    name: S.literal('_cf_KV'),
    type: S.literal('table'),
  }),
  S.struct({
    schema: S.literal('main'),
    name: S.literal('sqlite_schema'),
    type: S.literal('table'),
  }),
)
export type D1Table = S.Schema.Type<typeof D1Table>

export const D1Tables = S.array(D1Table)
