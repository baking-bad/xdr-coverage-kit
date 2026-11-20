import { isDeepStrictEqual } from "node:util";
import { type Sdk, type XdrType, sameBytes, xdrType } from "./sdk.ts";
import { type Check, Refused, sdkCall } from "./verdict.ts";

/** The checks that work on any XDR type with no code per type. */
export function levels(sdk: Sdk): Check[] {
  return [codec(sdk), json(sdk)];
}

/** Bytes to an SDK object and back to bytes. */
function codec(sdk: Sdk): Check {
  return {
    name: "codec",
    lost: "the SDK writes other bytes",
    covers: every,
    keeps(item) {
      const type = typeOf(sdk, item.xdrType);
      return sdkCall(() => sameBytes(type.fromXdr(item.bytes), item.bytes));
    },
  };
}

/**
 * Bytes to XDR-JSON (SEP-0051), compared with the decoded value of the
 * corpus, and that value back to bytes.
 */
function json(sdk: Sdk): Check {
  return {
    name: "json",
    lost: "the SDK writes other XDR-JSON or other bytes",
    covers: every,
    keeps(item) {
      const type = typeOf(sdk, item.xdrType);
      return sdkCall(
        () =>
          isDeepStrictEqual(type.fromXdr(item.bytes).toJson(), item.decoded) &&
          sameBytes(type.fromJson(item.decoded), item.bytes),
      );
    },
  };
}

const every = () => true;

function typeOf(sdk: Sdk, name: string): XdrType {
  const type = xdrType(sdk, name);
  if (!type) throw new Refused(`the SDK has no XDR type ${name}`);
  return type;
}
