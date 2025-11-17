
export const wasmFile = (contractName: string) => {
  let fileStr = contractName
  fileStr += '.wasm'
  return fileStr
}
