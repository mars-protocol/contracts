import { readFileSync, writeFileSync } from 'fs';
import path from 'path';
export const ARTIFACTS_PATH = '../artifacts';
export function writeArtifact(data, name = 'artifact') {
  writeFileSync(path.join(ARTIFACTS_PATH, `${name}.json`), JSON.stringify(data, null, 2));
}
// Reads json containing contract addresses located in /artifacts folder for specified network.
export function readArtifact(name = 'artifact') {
  try {
    const data = readFileSync(path.join(ARTIFACTS_PATH, `${name}.json`), 'utf8');
    return JSON.parse(data);
  } catch (e) {
    return {};
  }
}
