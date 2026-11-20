import fs from "node:fs";

/**
 * An error about the input of the runner: its arguments, the corpus or a file
 * of known verdicts. The runner prints its message alone. Any other error is
 * a defect of the runner and prints with its stack.
 */
export class InputError extends Error {}

/** The bytes of `file`. Throws `InputError` when the file cannot be read. */
export function readFile(file: string): Buffer {
  try {
    return fs.readFileSync(file);
  } catch (error) {
    throw new InputError(`${file}: ${messageOf(error)}`);
  }
}

/** The JSON value in `file`. Throws `InputError` when it is not JSON. */
export function readJson(file: string): unknown {
  const text = readFile(file).toString("utf8");
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new InputError(`${file}: ${messageOf(error)}`);
  }
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
