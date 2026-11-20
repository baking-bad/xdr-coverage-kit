import { isDeepStrictEqual } from "node:util";
import { type Item, type Json, isObject } from "./corpus.ts";
import { type Sdk, type XdrValue, sameBytes, xdrType } from "./sdk.ts";
import { type Check, Refused, Skipped, sdkCall } from "./verdict.ts";

const ofType = (type: string) => (item: Item) => item.xdrType === type;

/** The note of the verdict `lost` of every check that builds a value again. */
const REBUILT = "the rebuilt value has other bytes";

/** The checks of the functions of the SDK that an application calls. */
export function helpers(sdk: Sdk): Check[] {
  return [
    scValToNative(sdk),
    unionValue(sdk),
    contractSpec(sdk),
    memoValue(sdk),
    rebuild(sdk, "rebuild", (fields) => fields),
    rebuild(sdk, "rebuild(numbers)", integersAsNumbers),
    cloneFrom(sdk),
  ];
}

/**
 * Reads a Soroban value into a value of the language and compares the whole
 * result with the decoded value.
 */
function scValToNative(sdk: Sdk): Check {
  return {
    name: "scValToNative",
    lost: "the native value differs from the decoded value",
    covers: ofType("ScVal"),
    keeps(item) {
      const expected = expectedNative(item.decoded);
      const native: unknown = sdkCall(() =>
        sdk.scValToNative(sdk.xdr.ScVal.fromXdr(item.bytes)),
      );
      return isDeepStrictEqual(withBuffers(native), expected);
    },
  };
}

/**
 * Rebuilds a generated union from the name of its arm and its `value`,
 * and compares the bytes. The helper serves every union type of the SDK
 * with no code for one type: a memo, a Soroban value, an asset.
 */
function unionValue(sdk: Sdk): Check {
  /** The union of an item and the constructor of its arm, if it has them. */
  const read = (item: Item) => {
    const type = xdrType(sdk, item.xdrType);
    if (!type) return undefined;
    const union = sdkCall(() => type.fromXdr(item.bytes));
    const arm = typeof union.type === "string" ? type[union.type] : null;
    if (typeof arm !== "function") return undefined;
    return { union, build: arm as (value?: unknown) => XdrValue };
  };
  return {
    name: "union.value",
    lost: REBUILT,
    covers(item) {
      // NOTE: a release that cannot read the item gives no union,
      // and the level `codec` reports that refusal.
      try {
        return read(item) !== undefined;
      } catch (error) {
        if (error instanceof Refused) return false;
        throw error;
      }
    },
    keeps(item) {
      const parts = read(item);
      if (!parts) throw new Skipped(`${item.xdrType} is no union`);
      const { union, build } = parts;
      return sdkCall(() => {
        const rebuilt =
          union.value === undefined ? build() : build(union.value);
        return sameBytes(rebuilt, item.bytes);
      });
    },
  };
}

/** A type of a contract spec, which `contract.Spec` reads a value by. */
type SpecType = Parameters<
  Sdk["contract"]["Spec"]["prototype"]["scValToNative"]
>[1];

/**
 * Reads a Soroban value through `contract.Spec` by its type and builds it
 * again, as a generated contract client does, and compares the bytes.
 * The type comes from the value itself: the arm `scvString` has the type
 * `scSpecTypeString`. So the helper serves every Soroban value of one
 * type, with no code for one type. It skips a vector, a map, and a value
 * whose arm has no spec type.
 */
function contractSpec(sdk: Sdk): Check {
  const types = sdk.xdr.ScSpecTypeDef as unknown as Record<
    string,
    (() => SpecType) | undefined
  >;
  // NOTE: a spec needs one entry, and the helper reads none of them.
  const spec = new sdk.contract.Spec([
    sdk.xdr.ScSpecEntry.scSpecEntryFunctionV0(
      new sdk.xdr.ScSpecFunctionV0({
        doc: "",
        name: "f",
        inputs: [],
        outputs: [],
      }),
    ),
  ]);
  const typeOf = (arm: string): SpecType => {
    if (arm === "scvVec" || arm === "scvMap") {
      throw new Skipped(
        "the runner gives no spec type to a vector or a map, " +
          "whose values can have several types",
      );
    }
    const type = types[arm.replace("scv", "scSpecType")];
    if (!type) throw new Skipped(`the SDK has no spec type for ${arm}`);
    return type();
  };
  return {
    name: "contract.Spec",
    lost: REBUILT,
    covers: ofType("ScVal"),
    keeps(item) {
      const value = sdkCall(() => sdk.xdr.ScVal.fromXdr(item.bytes));
      const type = typeOf(value.type);
      return sdkCall(() => {
        const native: unknown = spec.scValToNative(value, type);
        return sameBytes(spec.nativeToScVal(native, type), item.bytes);
      });
    },
  };
}

/** Reads a memo into the class `Memo` and builds it again. */
// NOTE: this check reads the value of the class `Memo`, which wraps
// the generated union that `union.value` reads.
function memoValue(sdk: Sdk): Check {
  return {
    name: "Memo.value",
    lost: REBUILT,
    covers: ofType("Memo"),
    keeps(item) {
      return sdkCall(() => {
        const memo = sdk.Memo.fromXdrObject(sdk.xdr.Memo.fromXdr(item.bytes));
        const rebuilt = new sdk.Memo(memo.type, memo.value);
        return sameBytes(rebuilt.toXdrObject(), item.bytes);
      });
    },
  };
}

/** The fields of an operation as the SDK reads them, with its kind. */
type Fields = { type: string } & Record<string, unknown>;

/**
 * Rebuilds an operation through the builders of `sdk` and compares the bytes
 * with the entry. The SDK reads the operation into its fields, and `pass`
 * turns them into the form that an application passes. The builder of that
 * kind then builds the operation again. `name` names the check, which skips
 * an operation with no builder. The helper names no builder and no field, so
 * it serves every kind of operation.
 */
function rebuild(
  sdk: Sdk,
  name: string,
  pass: (fields: Fields) => Fields,
): Check {
  const builders = sdk.Operation as unknown as Record<
    string,
    ((fields: Fields) => XdrValue) | undefined
  >;
  return {
    name,
    lost: REBUILT,
    covers: ofType("Operation"),
    keeps(item) {
      const fields = sdkCall(() => {
        const operation = sdk.xdr.Operation.fromXdr(item.bytes);
        return sdk.Operation.fromXdrObject(operation) as Fields;
      });
      const build = builders[fields.type];
      if (!build)
        throw new Skipped(`the SDK has no operation builder ${fields.type}`);
      const passed = pass(fields);
      return sdkCall(() => sameBytes(build(passed), item.bytes));
    },
  };
}

/** Lets the SDK read a transaction, clone it and build it again. */
// NOTE: `rpc.assembleTransaction` clones a transaction this way
// before the application signs it.
function cloneFrom(sdk: Sdk): Check {
  return {
    name: "cloneFrom",
    lost: REBUILT,
    covers: ofType("Transaction"),
    keeps(item) {
      return sdkCall(() => {
        const envelope = sdk.xdr.TransactionEnvelope.envelopeTypeTx(
          new sdk.xdr.TransactionV1Envelope({
            tx: sdk.xdr.Transaction.fromXdr(item.bytes),
            signatures: [],
          }),
        );
        // NOTE: the passphrase enters the hash of a transaction and
        // not its bytes, so any passphrase serves the comparison.
        const read = new sdk.Transaction(envelope, sdk.Networks.STANDALONE);
        const clone = sdk.TransactionBuilder.cloneFrom(read).build();
        return sameBytes(clone.tx, item.bytes);
      });
    },
  };
}

/**
 * The fields with every string of digits passed as a number. The SDK returns
 * a 64-bit integer, such as an offer ID, as a string of digits. An
 * application may pass it as a JavaScript number, which holds no integer
 * above 2^53 exactly. An amount comes as a string with a decimal point, and
 * stays a string.
 */
// NOTE: the pass knows no field, so it also turns other strings of digits
// into numbers. The builder takes a price such as `"1"` as a number, and the
// SDK refuses a number in a text field.
function integersAsNumbers(fields: Fields): Fields {
  return Object.fromEntries(
    Object.entries(fields).map(([key, value]) => [
      key,
      typeof value === "string" && /^\d+$/.test(value) ? Number(value) : value,
    ]),
  ) as Fields;
}

/**
 * The native value that keeps a decoded `ScVal` whole: the form that
 * `scValToNative` documents for each kind of value. Throws `Skipped`
 * for a kind that has no such form.
 */
export function expectedNative(decoded: Json): unknown {
  if (decoded === "void") return null;
  const [[arm, value] = []] = isObject(decoded) ? Object.entries(decoded) : [];
  switch (arm) {
    case "bool":
    case "u32":
    case "i32":
    case "symbol":
    case "address":
      return value;
    case "u64":
    case "i64":
    case "u128":
    case "i128":
    case "u256":
    case "i256":
    case "timepoint":
    case "duration":
      return BigInt(value as string);
    case "bytes":
      return Buffer.from(value as string, "hex");
    case "string": {
      // NOTE: `scValToNative` returns a string that is not UTF-8 as its bytes.
      const bytes = unescaped(value as string);
      return utf8(bytes) ?? bytes;
    }
    case "vec":
      return items(value).map(expectedNative);
    case "map": {
      const pairs = items(value).map((entry): [unknown, unknown] => {
        const [key, val] = pair(entry);
        return [expectedNative(key), expectedNative(val)];
      });
      // NOTE: `scValToNative` returns a map as an object, and an object key
      // is a string. Keys that print the same, or a key with no text, cannot
      // all be keys of one object. So only a `Map` would keep such a value.
      // `scValToNative` never returns one, so its best verdict for such a
      // map is `refused`.
      const names = pairs.map(([key]) => keyName(key));
      return names.every((name) => name !== undefined) &&
        new Set(names).size === pairs.length
        ? Object.fromEntries(pairs.map(([, val], at) => [names[at], val]))
        : new Map(pairs);
    }
    default:
      throw new Skipped(`scValToNative has no documented form for ${arm}`);
  }
}

/**
 * The items of a vector or a map. XDR lets either one be absent, and an
 * absent one throws `Skipped`.
 */
function items(value: Json | undefined): Json[] {
  if (!Array.isArray(value)) {
    throw new Skipped(
      "scValToNative has no documented form for an absent vector or map",
    );
  }
  return value;
}

/**
 * The JSON of a map entry: a key and a value. Throws `Skipped` when either
 * one is missing.
 */
function pair(entry: Json): [Json, Json] {
  if (!isObject(entry) || entry.key === undefined || entry.val === undefined) {
    throw new Skipped("a pair of the decoded map has no key or no val");
  }
  return [entry.key, entry.val];
}

/** The name that an object gives to a key, if the key has one. */
function keyName(key: unknown): string | undefined {
  if (Buffer.isBuffer(key)) return utf8(key);
  return ["string", "number", "bigint", "boolean"].includes(typeof key)
    ? String(key)
    : undefined;
}

/** The text of bytes that are UTF-8, and `undefined` for other bytes. */
function utf8(bytes: Buffer): string | undefined {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return undefined;
  }
}

/**
 * The bytes of a string of XDR-JSON, which writes a byte outside
 * printable ASCII as an escape such as `\xff`.
 */
function unescaped(text: string): Buffer {
  const short: Record<string, number> = { "0": 0, t: 9, n: 10, r: 13 };
  const bytes: number[] = [];
  for (let at = 0; at < text.length; at += 1) {
    const char = text.charAt(at);
    if (char !== "\\") {
      bytes.push(...Buffer.from(char, "utf8"));
      continue;
    }
    const next = text.charAt(at + 1);
    if (next === "x") {
      bytes.push(parseInt(text.slice(at + 2, at + 4), 16));
      at += 3;
    } else {
      bytes.push(short[next] ?? next.charCodeAt(0));
      at += 1;
    }
  }
  return Buffer.from(bytes);
}

/** The value with every byte array as a `Buffer`, which compares by value. */
function withBuffers(value: unknown): unknown {
  if (value instanceof Uint8Array) return Buffer.from(value);
  if (Array.isArray(value)) return value.map(withBuffers);
  if (value instanceof Map || typeof value !== "object" || value === null) {
    return value;
  }
  return Object.fromEntries(
    Object.entries(value).map(([key, inner]) => [key, withBuffers(inner)]),
  );
}
