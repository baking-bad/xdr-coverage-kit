import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { expect, test } from "vitest";
import { readCorpus } from "../src/corpus.ts";

const CORPUS = path.join(import.meta.dirname, "..", "..", "..", "corpus");

test("readCorpus refuses a manifest that the schema refuses", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "corpus-"));
  fs.copyFileSync(
    path.join(CORPUS, "manifest.schema.json"),
    path.join(dir, "manifest.schema.json"),
  );
  fs.writeFileSync(path.join(dir, "manifest.json"), '{"sources": []}');
  expect(() => readCorpus(dir)).toThrow("manifest.json");
});
