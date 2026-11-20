/**
 * Runs the corpus through one JavaScript SDK.
 * Usage: node src/run.ts [path to an SDK package] [--known file].
 * Default: the installed SDK, and no known verdicts.
 * Exit code:
 * 0: every verdict is kept, skipped, or lost or refused as the file names it.
 * 1: a value lost or refused that the file does not name, or a verdict of
 *    the file that the run did not give.
 * 2: the runner failed.
 */
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { parseArgs } from "node:util";
import { type Item, readCorpus } from "./corpus.ts";
import { helpers } from "./helpers.ts";
import { InputError } from "./input.ts";
import { readKnown } from "./known.ts";
import { levels } from "./levels.ts";
import { type Checked, printReport } from "./report.ts";
import type { Sdk } from "./sdk.ts";
import type { Check } from "./verdict.ts";

const HERE = import.meta.dirname;
const PACKAGE = path.join(HERE, "..");
const CORPUS = path.join(PACKAGE, "..", "..", "corpus");
const INSTALLED = path.join(PACKAGE, "node_modules", "@stellar/stellar-sdk");

// NOTE: the SDK is loaded from a directory that the user names, and only
// `require` resolves a package by its directory.
const require = createRequire(import.meta.url);

const USAGE =
  "Usage: npm start -- [directory of an SDK package] " +
  "[--known file of known verdicts]";

function main(): void {
  const { values, positionals } = parseArguments();
  if (values.help) {
    console.log(USAGE);
    return;
  }
  const sdkPath = positionals[0] ? path.resolve(positionals[0]) : INSTALLED;
  const known = readKnown(values.known);
  if (!fs.existsSync(path.join(sdkPath, "package.json"))) {
    throw new InputError(
      `no SDK package at ${sdkPath}: ` +
        "name the directory of an installed @stellar/stellar-sdk",
    );
  }
  const sdk = require(sdkPath) as Sdk;
  const { sources, entries } = readCorpus(CORPUS);

  const levelChecks = levels(sdk);
  const allChecks = [...levelChecks, ...helpers(sdk)];
  const read = (item: Item, checks: Check[]): Checked => ({
    item,
    checks: checks.filter((check) => check.covers(item)),
  });

  const from = values.known
    ? `known verdicts from ${path.relative(process.cwd(), values.known)}`
    : "no known verdicts";
  console.log(`@stellar/stellar-sdk ${packageVersion(sdkPath)}, ${from}\n`);
  // NOTE: a source goes through the levels only, because the helpers judge
  // the shapes, and each shape is an entry. `union.value` would also read a
  // ledger, which is a union.
  const failures = printReport(
    [
      ...sources.map((source) => read(source, levelChecks)),
      ...entries.map((entry) => read(entry, allChecks)),
    ],
    known,
  );
  process.exitCode = failures === 0 ? 0 : 1;
}

/** The arguments of the command line. Throws `InputError` on a wrong one. */
function parseArguments() {
  try {
    return parseArgs({
      options: {
        known: { type: "string" },
        help: { type: "boolean" },
      },
      allowPositionals: true,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new InputError(`${message}\n${USAGE}`);
  }
}

function packageVersion(dir: string): string {
  const file = fs.readFileSync(path.join(dir, "package.json"), "utf8");
  return (JSON.parse(file) as { version: string }).version;
}

try {
  main();
} catch (error) {
  console.error(
    error instanceof InputError ? `Error: ${error.message}` : error,
  );
  process.exitCode = 2;
}
