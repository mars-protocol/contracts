import { readFile, writeFile } from 'fs/promises'
import path from 'path'
import { StorageItems as StorageItems } from '../../types/storageItems'

export const ARTIFACTS_PATH = '../artifacts/'

export class Storage implements StorageItems {
  public addresses: StorageItems['addresses']
  public codeIds: StorageItems['codeIds']
  public actions: StorageItems['actions']

  constructor(
    private chainId: string,
    private label: string,
    items: StorageItems,
  ) {
    this.addresses = items.addresses
    this.codeIds = items.codeIds
    this.actions = items.actions
  }

  static async load(chainId: string, label: string): Promise<Storage> {
    try {
      const data = await readFile(path.join(ARTIFACTS_PATH, `${chainId}-${label}.json`), 'utf8')
      const items = JSON.parse(data) as StorageItems
      return new this(chainId, label, items)
    } catch (e) {
      return new this(chainId, label, {
        addresses: {},
        codeIds: {},
        actions: {
          addressProviderSet: {},
          redBankMarketsSet: [],
          assetsSet: [],
          vaultsSet: [],
          oraclePricesSet: [],
          routesSet: [],
        },
      })
    }
  }

  async save() {
    await writeFile(
      path.join(ARTIFACTS_PATH, `${this.chainId}-${this.label}.json`),
      JSON.stringify(this, null, 2),
    )
  }
}
