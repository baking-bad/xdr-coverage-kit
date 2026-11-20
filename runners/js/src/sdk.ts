/** The SDK as the installed release types it. */
export type Sdk = typeof import("@stellar/stellar-sdk");

/** A value of the SDK that encodes itself to XDR. */
export interface XdrValue {
  toXdr(): Uint8Array;
}

/** A value that a generated XDR type reads from bytes. */
interface XdrObject extends XdrValue {
  toJson(): unknown;
  /** The name of the arm, when the type is a union. */
  type?: unknown;
  value?: unknown;
}

/**
 * The part of a generated XDR type that the runner uses. A union type
 * also has one constructor for each arm, by its name.
 */
export interface XdrType {
  fromXdr(bytes: Buffer): XdrObject;
  fromJson(json: unknown): XdrValue;
  [arm: string]: unknown;
}

/** The generated XDR type of this name, if the SDK has one. */
export function xdrType(sdk: Sdk, name: string): XdrType | undefined {
  const types = sdk.xdr as unknown as Record<string, XdrType | undefined>;
  return types[name];
}

/** Whether `value` encodes to exactly `bytes`. */
export function sameBytes(value: XdrValue, bytes: Buffer): boolean {
  return Buffer.from(value.toXdr()).equals(bytes);
}
