import { get_config, get_dmmf } from './prisma_fmt_build'

async function main() {
  const input = {
    useGetConfig: true,
    useGetDMMF: true,
  }

  const schema = /* prisma */ `
    generator client {
      provider        = "prisma-client-js"
      previewFeatures = ["referentialIntegrity"]
    }

    datasource db {
      provider     = "postgres"
      url          = env("TEST_POSTGRES_URI")
      relationMode = "foreignKeys"
    }

    model Profile {
      id     Int  @id
      userId Int  @unique @default(1)
      user   User @relation(fields: [userId], references: [id], onUpdate: SetDefault)
    }

    model User {
      id      Int      @id
      profile Profile?
    }
  `

  if (input.useGetConfig) {
    const getConfigParams = JSON.stringify({
      prismaSchema: schema,
      datasourceOverrides: {},
      ignoreEnvVarErrors: true,
      // @ts-ignore
      env: process.env,
    })
    const config = get_config(getConfigParams)
    
    console.log('config', config)
  }

  // note: if the Prisma schema is not valid, this panics
  if (input.useGetDMMF) {
    const getDMMFParams = JSON.stringify({
      prismaSchema: schema,
    })
    const dmmf = get_dmmf(getDMMFParams)
    const dmmfAsJSON = JSON.parse(dmmf)
  
    console.log('dmmf', dmmfAsJSON)
  }
}

main()