import * as S from '@effect/schema/Schema'

const EnvPlanetScale = S.struct({
  DRIVER_ADAPTER: S.literal('planetscale'),
  DRIVER_ADAPTER_CONFIG: S.struct({
    proxy_url: S.string,
  }),
})

const EnvNeonWS = S.struct({
  DRIVER_ADAPTER: S.literal('neon:ws'),
  DRIVER_ADAPTER_CONFIG: S.struct({
    url: S.string,
  }),
})

export const ExternalTestExecutor = S.literal('Wasm', 'Napi')
export type ExternalTestExecutor = S.Schema.Type<typeof ExternalTestExecutor>

export const Env = S.extend(
  S.union(
    EnvPlanetScale,
    EnvNeonWS,
    S.struct({
      DRIVER_ADAPTER: S.literal('pg', 'libsql'),
    }),
  ),
  S.struct({
    EXTERNAL_TEST_EXECUTOR: S.optional(ExternalTestExecutor),
  }),
)

export type Env = S.Schema.Type<typeof Env>

export type DriverAdapterTag = Env['DRIVER_ADAPTER']
