import { stripVTControlCharacters } from "node:util";
import { afterEach, expect, test, vi } from "vitest";
import type { Item } from "../src/corpus.ts";
import type { KnownItem } from "../src/known.ts";
import { type Checked, printReport } from "../src/report.ts";
import { type Check, Refused } from "../src/verdict.ts";

const item: Item = {
  id: "memo",
  xdrType: "Memo",
  bytes: Buffer.of(),
  decoded: null,
};
const keeps = () => true;
const loses = () => false;
const refuses = () => {
  throw new Refused("Error: no");
};
const check = (name: string, keeps: Check["keeps"]): Check => ({
  name,
  lost: "the SDK writes other bytes",
  covers: () => true,
  keeps,
});

/**
 * Runs the report and returns its failures and its lines. The lines come
 * without color codes, so a test reads the same text with and without colors.
 */
function report(checks: Checked["checks"], list: KnownItem[] = []) {
  const log = vi.spyOn(console, "log").mockImplementation(() => {});
  const failures = printReport([{ item, checks }], list);
  const lines = log.mock.calls.map(([line]) =>
    stripVTControlCharacters(String(line)),
  );
  return { failures, lines };
}

afterEach(() => vi.restoreAllMocks());

test("printReport fails a lost value and a refused value", () => {
  const { failures, lines } = report([
    check("codec", loses),
    check("json", refuses),
  ]);
  expect(failures).toBe(2);
  expect(lines[1]).toBe("refused          memo  json   Error: no");
  expect(lines.at(-2)).toContain("2 checks: 1 lost, 1 refused");
  expect(lines.at(-1)).toContain("2 failures");
});

test("printReport fails a check whose verdict differs from its known verdict", () => {
  const list: KnownItem[] = [
    { id: "memo", check: "codec", verdict: "lost", reason: "#1823" },
  ];
  const kept = report([check("codec", keeps)], list);
  expect(kept.failures).toBe(1);
  expect(kept.lines[0]).toContain("the file of known verdicts states lost");
  const refused = report([check("codec", refuses)], list);
  expect(refused.failures).toBe(1);
});

test("printReport fails a known verdict for a check that the run does not have", () => {
  const list: KnownItem[] = [
    { id: "memo", check: "no-such-check", verdict: "lost", reason: "#1823" },
  ];
  const { failures, lines } = report([check("codec", keeps)], list);
  expect(failures).toBe(1);
  expect(lines.at(-2)).toContain("memo  no-such-check");
});
