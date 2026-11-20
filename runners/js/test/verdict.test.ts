import { expect, test } from "vitest";
import type { Item } from "../src/corpus.ts";
import { Skipped, verdict } from "../src/verdict.ts";

const item: Item = {
  id: "x",
  xdrType: "Memo",
  bytes: Buffer.of(),
  decoded: null,
};

test("verdict skips a check that cannot judge, with its reason", () => {
  const check = {
    name: "x",
    lost: "the SDK writes other bytes",
    covers: () => true,
    keeps: () => {
      throw new Skipped("no such type");
    },
  };
  expect(verdict(check, item)).toEqual({
    verdict: "skipped",
    note: "no such type",
  });
});

test("verdict throws an error of the runner itself", () => {
  const check = {
    name: "x",
    lost: "the SDK writes other bytes",
    covers: () => true,
    keeps: () => {
      throw new TypeError("a defect of the runner");
    },
  };
  expect(() => verdict(check, item)).toThrow(TypeError);
});
