import { styleText } from "node:util";
import type { Item } from "./corpus.ts";
import type { KnownItem } from "./known.ts";
import { type Check, verdict } from "./verdict.ts";

/** One source or one entry with the checks that read it. */
export interface Checked {
  item: Item;
  checks: Check[];
}

type Color = "green" | "yellow" | "red" | "gray";

/** The labels of the report in the order of the summary. */
const LABELS = [
  "kept",
  "lost",
  "lost (known)",
  "refused",
  "refused (known)",
  "skipped",
];

/**
 * Prints one line for each check of each item: the verdict, the item,
 * the check and the reason. Returns the count of failures. A `lost` or
 * `refused` verdict fails unless `known` names it. A verdict that
 * `known` names fails when the check gives another, and when no check
 * of the run stands at its place.
 */
export function printReport(all: Checked[], known: KnownItem[]): number {
  const places = all.flatMap(({ item, checks }) =>
    checks.map((check) => ({ item, check })),
  );
  const unseen = known.filter(
    (listed) =>
      !places.some(
        ({ item, check }) =>
          item.id === listed.id && check.name === listed.check,
      ),
  );
  const longest = (texts: string[]) =>
    Math.max(...texts.map((text) => text.length));
  const idWidth =
    longest([
      ...places.map(({ item }) => item.id),
      ...unseen.map(({ id }) => id),
    ]) + 2;
  const nameWidth =
    longest([
      ...places.map(({ check }) => check.name),
      ...unseen.map(({ check }) => check),
    ]) + 2;
  const labelWidth = longest(LABELS) + 2;
  const print = (
    label: string,
    color: Color,
    id: string,
    name: string,
    reason: string,
  ) => {
    // NOTE: the padding stands outside the color codes, so the columns
    // keep their width with and without colors.
    const cell = paint(color, label) + " ".repeat(labelWidth - label.length);
    const place = id.padEnd(idWidth) + name.padEnd(nameWidth);
    console.log(`${cell}${place}${reason}`.trimEnd());
  };
  const counts = new Map<string, number>();
  let failures = 0;

  for (const { item, check } of places) {
    const result = verdict(check, item);
    const expected = known.find(
      (listed) => listed.id === item.id && listed.check === check.name,
    );
    const note = "note" in result ? result.note : "";
    let line: [label: string, color: Color, reason: string];
    if (expected?.verdict === result.verdict) {
      const reason =
        result.verdict === "refused"
          ? `${expected.reason}. SDK: ${note}`
          : expected.reason;
      line = [`${result.verdict} (known)`, "yellow", reason];
    } else if (expected) {
      const states = `the file of known verdicts states ${expected.verdict}`;
      line = [result.verdict, "red", note ? `${states}: ${note}` : states];
    } else if (result.verdict === "kept") {
      line = ["kept", "green", ""];
    } else if (result.verdict === "skipped") {
      line = ["skipped", "gray", note];
    } else {
      line = [result.verdict, "red", note];
    }
    const [label, color, reason] = line;
    if (color === "red") failures += 1;
    counts.set(label, (counts.get(label) ?? 0) + 1);
    print(label, color, item.id, check.name, reason);
  }

  const parts = LABELS.filter((label) => counts.has(label)).map(
    (label) => `${counts.get(label)} ${label}`,
  );
  console.log(`\n${places.length} checks: ${parts.join(", ")}\n`);
  for (const { id, check } of unseen) {
    print(
      "fail",
      "red",
      id,
      check,
      "the run gives no verdict for this check: " +
        "remove it from the file of known verdicts",
    );
  }
  failures += unseen.length;
  if (failures > 0) {
    const noun = failures === 1 ? "failure" : "failures";
    console.log(
      `${failures} ${noun}: each lost or refused line without (known), ` +
        "each line that states another known verdict, and each fail line\n",
    );
  }
  return failures;
}

/** Colors a word when the output shows colors. */
function paint(color: Color, text: string): string {
  return styleText(color, text, { validateStream: true });
}
