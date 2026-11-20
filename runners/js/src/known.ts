import { InputError, readJson } from "./input.ts";

/**
 * One known verdict: the SDK under test gives `verdict` for the item `id` at
 * the check `check`. `reason` says why the team accepts the verdict for now.
 */
export interface KnownItem {
  id: string;
  check: string;
  verdict: "lost" | "refused";
  reason: string;
}

/**
 * Reads the known verdicts from the JSON file `file`, or returns an empty
 * list when there is no file. Throws when the file holds anything other than
 * a list of known verdicts, or lists one place twice.
 */
export function readKnown(file: string | undefined): KnownItem[] {
  const list: unknown = file ? readJson(file) : [];
  if (!Array.isArray(list)) {
    throw new InputError(
      `${file}: the file must hold a list of known verdicts`,
    );
  }
  const wrong = list.findIndex((item) => !isKnownItem(item));
  if (wrong !== -1) {
    throw new InputError(
      `${file}: known verdict ${wrong + 1} must hold id, check and reason ` +
        "as text, and verdict as lost or refused",
    );
  }
  const known = list as KnownItem[];
  const twice = known.find(
    (item, at) =>
      known.findIndex(
        (other) => other.id === item.id && other.check === item.check,
      ) !== at,
  );
  if (twice) {
    throw new InputError(
      `${file}: the check ${twice.check} of ${twice.id} stands twice`,
    );
  }
  return known;
}

function isKnownItem(value: unknown): value is KnownItem {
  return (
    typeof value === "object" &&
    value !== null &&
    "id" in value &&
    typeof value.id === "string" &&
    "check" in value &&
    typeof value.check === "string" &&
    "reason" in value &&
    typeof value.reason === "string" &&
    "verdict" in value &&
    (value.verdict === "lost" || value.verdict === "refused")
  );
}
