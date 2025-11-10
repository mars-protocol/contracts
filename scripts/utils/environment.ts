// import os from 'os'

// for m1 macs, the binaries should look like: rover-aarch64.wasm vs rover.wasm
export const wasmFile = (contractName: string) => {
  let fileStr = contractName
  // const env = os.arch()
  // if (env === 'arm64') {
  //   fileStr += ''
  // }
  fileStr += '.wasm'
  return fileStr
}
