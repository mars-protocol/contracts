import codegen from '@cosmwasm/ts-codegen'
import { join, resolve } from 'path'
import { printGreen, printRed } from '../utils/chalk'
import { readdir } from 'fs/promises'

void (async function () {
  const schemasDir = resolve(join(__dirname, '../../../schemas'))
  const schemas = await readdir(schemasDir)

  for (const schema of schemas) {
    try {
      await codegen({
        contracts: [`${schemasDir}/${schema}`],
        outPath: `./types/generated/${schema}`,
        options: {
          types: {
            enabled: true,
          },
          client: {
            enabled: true,
          },
          reactQuery: {
            enabled: true,
            optionalClient: true,
            version: 'v4',
            mutations: true,
            queryKeys: true,
          },
          messageComposer: {
            enabled: false,
          },
        },
      })
      printGreen(`Success ✨ ${schema} types generated`)
    } catch (e) {
      printRed(`Error with ${schema}: ${e}`)
    }
  }
})()
