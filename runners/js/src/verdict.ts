import type { Item } from "./corpus.ts";

/**
 * A check of the SDK, a level or a helper. `covers` says whether the check
 * reads an item, and `keeps` runs the SDK on it and answers whether the
 * value survived. `keeps` throws `Skipped` when the check cannot judge, and
 * `Refused` through `sdkCall` when the SDK raises an error.
 */
export interface Check {
  name: string;
  /** The note of the verdict `lost`: what differs from the corpus. */
  lost: string;
  covers(item: Item): boolean;
  keeps(item: Item): boolean;
}

export type Verdict =
  | { verdict: "kept" }
  | { verdict: "lost" | "refused" | "skipped"; note: string };

/** A check throws this when it cannot judge the value. */
export class Skipped extends Error {}

/** An error that the SDK raised. Any other error is a defect of the runner. */
export class Refused extends Error {}

/**
 * Runs a call into the SDK and rethrows its error as `Refused`. The call
 * must hold code of the SDK alone, because an error of the runner inside it
 * would pass as a verdict about the SDK.
 */
export function sdkCall<T>(call: () => T): T {
  try {
    return call();
  } catch (error) {
    throw new Refused(
      error instanceof Error
        ? `${error.name}: ${error.message}`
        : String(error),
    );
  }
}

/**
 * Runs one check on one item and names the outcome.
 * `kept`: the SDK kept the value. `lost`: it returned another value with
 * no error. `refused`: it raised an error. `skipped`: the check could not
 * judge the value. An error of the runner itself is not a verdict, and the
 * function throws it again.
 */
export function verdict(check: Check, item: Item): Verdict {
  try {
    return check.keeps(item)
      ? { verdict: "kept" }
      : { verdict: "lost", note: check.lost };
  } catch (error) {
    if (error instanceof Skipped) {
      return { verdict: "skipped", note: error.message };
    }
    if (error instanceof Refused) {
      return { verdict: "refused", note: error.message };
    }
    throw error;
  }
}
