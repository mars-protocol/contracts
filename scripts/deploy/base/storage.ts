import { readFile, writeFile } from 'fs/promises'
import path from 'path'
import { StorageItems } from '../../types/storageItems'

export const ARTIFACTS_PATH = '../artifacts/'

export class Storage implements StorageItems {
  public addresses: StorageItems['addresses']
  public codeIds: StorageItems['codeIds']
  public execute: StorageItems['execute']
  public owner: StorageItems['owner']
  private readonly chainId: string

  constructor(chainId: string, items: StorageItems) {
    this.addresses = items.addresses
    this.codeIds = items.codeIds
    this.execute = items.execute
    this.owner = items.owner
    this.chainId = chainId
  }

  static async load(chainId: string): Promise<Storage> {
    try {
      const data = await readFile(path.join(ARTIFACTS_PATH, `${chainId}.json`), 'utf8')
      const items = JSON.parse(data) as StorageItems
      return new this(chainId, items)
    } catch (e) {
      return new this(chainId, {
        addresses: {},
        codeIds: {},
        execute: { assetsInitialized: [] },
      })
    }
  }

  async save() {
    await writeFile(
      path.join(ARTIFACTS_PATH, `${this.chainId}.json`),
      JSON.stringify(this, null, 2),
    )
  }
}
