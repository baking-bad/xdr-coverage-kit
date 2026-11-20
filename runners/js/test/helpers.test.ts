import { expect, test } from "vitest";
import { expectedNative } from "../src/helpers.ts";

test("expectedNative reads wide integers, bytes and strings", () => {
  expect(expectedNative({ u64: "9007199254740993" })).toBe(9007199254740993n);
  expect(expectedNative({ bytes: "00ff" })).toEqual(Buffer.of(0, 255));
  expect(expectedNative({ string: "a\\nb" })).toBe("a\nb");
  expect(expectedNative({ string: "\\xc3\\xa9" })).toBe("é");
  expect(expectedNative("void")).toBe(null);
});

test("expectedNative reads a string that is not UTF-8 as its bytes", () => {
  expect(expectedNative({ string: "a\\xff" })).toEqual(Buffer.of(97, 255));
});
