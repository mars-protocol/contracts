import { readFile, writeFile } from 'fs/promises'
import path from 'path'
import { StorageItems as StorageItems } from '../../types/NEW_storageItems'

export const ARTIFACTS_PATH = '../artifacts/'

export class Storage implements StorageItems {
  public addresses: StorageItems['addresses']
  public codeIds: StorageItems['codeIds']
  public actions: StorageItems['actions']
  public execute: StorageItems['execute']
  public owner: StorageItems['owner']

  constructor(
    private chainId: string,
    private label: string,
    items: StorageItems,
  ) {
    this.addresses = items.addresses
    this.codeIds = items.codeIds
    this.actions = items.actions
    this.execute = items.execute
    this.owner = items.owner
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
        actions: {},
        execute: {
          assetsUpdated: [],
          marketsUpdated: [],
          vaultsUpdated: [],
          addressProviderUpdated: {},
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
