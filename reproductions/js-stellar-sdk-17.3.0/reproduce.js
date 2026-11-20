/**
 * Reproduces js-stellar-sdk#1789 and #1785: runs the JS runner once per
 * release, each with the file of its known verdicts. 17.2.1 is the last
 * release before both fixes. Its file says that `scValToNative` loses the map
 * and `rebuild(numbers)` loses the offer ID. 17.3.0 ships both fixes, and its
 * file says that both checks refuse. The script fails when either release
 * gives a verdict that its file does not list, or changes one that it lists.
 */
import { spawnSync } from "node:child_process";
import path from "node:path";

const HERE = import.meta.dirname;
const runner = path.join(HERE, "..", "..", "runners", "js", "src", "run.ts");
const versions = ["17.2.1", "17.3.0"];
for (const version of versions) {
  const sdk = path.join(HERE, "node_modules", `sdk-${version}`);
  const file = `known-${version}.json`;
  const known = path.join(HERE, file);
  const run = spawnSync(process.execPath, [runner, sdk, "--known", known], {
    stdio: "inherit",
  });
  if (run.status === 1) {
    console.error(
      `${version} gives a verdict other than ${file} states: ` +
        "see its report above",
    );
  } else if (run.status !== 0) {
    console.error(`the runner failed on ${version}: see the error above`);
  }
  if (run.status !== 0) process.exitCode = 1;
}
if (process.exitCode !== 1) {
  console.log(`${versions.join(" and ")} give the verdicts of their files`);
}
