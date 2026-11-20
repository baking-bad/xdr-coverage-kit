import path from "node:path";
import { Ajv2020 } from "ajv/dist/2020.js";
import { InputError, readFile, readJson as readJsonFile } from "./input.ts";

/** A value of XDR-JSON. */
export type Json =
  null | boolean | number | string | Json[] | { [key: string]: Json };

/** One source or one entry: an XDR value as bytes and as decoded JSON. */
export interface Item {
  id: string;
  xdrType: string;
  bytes: Buffer;
  decoded: Json;
}

interface Manifest {
  sources: { id: string; xdr_type: string }[];
  entries: {
    id: string;
    xdr_type: string;
    location: {
      source: string;
      json_pointer: string;
      byte_range: { offset: number; length: number };
    };
  }[];
}

/**
 * Reads the corpus in `dir`. An entry holds the bytes of its byte range and
 * the value at its JSON Pointer. Throws when the manifest fails its JSON
 * Schema. Throws also when an entry names a missing source, a byte range
 * outside its source or a path to no value.
 */
export function readCorpus(dir: string): { sources: Item[]; entries: Item[] } {
  const readBytes = (file: string): Buffer => readFile(path.join(dir, file));
  const readJson = (file: string): unknown =>
    readJsonFile(path.join(dir, file));
  const manifest = readManifest(readJson);

  const sources = manifest.sources.map((source) => ({
    id: source.id,
    xdrType: source.xdr_type,
    bytes: readBytes(path.join("sources", `${source.id}.xdr`)),
    decoded: readJson(path.join("sources", `${source.id}.json`)) as Json,
  }));
  const entries = manifest.entries.map((entry) => {
    const { json_pointer: pointer, byte_range: range } = entry.location;
    const id = entry.location.source;
    const source = sources.find((candidate) => candidate.id === id);
    if (!source) {
      throw new InputError(
        `entry ${entry.id}: the manifest has no source ${id}`,
      );
    }
    const end = range.offset + range.length;
    if (end > source.bytes.length) {
      throw new InputError(
        `entry ${entry.id}: the byte range ends at byte ${end}, ` +
          `after the ${source.bytes.length} bytes of the source`,
      );
    }
    return {
      id: entry.id,
      xdrType: entry.xdr_type,
      bytes: source.bytes.subarray(range.offset, range.offset + range.length),
      decoded: valueAt(source.decoded, pointer, entry.id),
    };
  });
  return { sources, entries };
}

/** Reads the manifest and checks it against its JSON Schema. */
function readManifest(readJson: (file: string) => unknown): Manifest {
  const manifest = readJson("manifest.json");
  const schema = readJson("manifest.schema.json") as object;
  const ajv = new Ajv2020();
  const validate = ajv.compile<Manifest>(schema);
  if (!validate(manifest)) {
    const errors = ajv.errorsText(validate.errors, { dataVar: "" });
    throw new InputError(
      `manifest.json does not match manifest.schema.json: ${errors}`,
    );
  }
  return manifest;
}

/** Whether `json` is a JSON object, that is neither null nor an array. */
export function isObject(json: Json): json is { [key: string]: Json } {
  return typeof json === "object" && json !== null && !Array.isArray(json);
}

/**
 * The value at a JSON Pointer (RFC 6901).
 * Throws when the path leaves the value.
 */
// NOTE: a key of XDR-JSON is the name of an XDR field or of a union arm.
// Neither holds `/` or `~`, so no key needs the escape of JSON Pointer.
function valueAt(json: Json, pointer: string, entry: string): Json {
  let value = json;
  for (const key of pointer.split("/").slice(1)) {
    const next = Array.isArray(value)
      ? /^\d+$/.test(key)
        ? value[Number(key)]
        : undefined
      : isObject(value) && Object.hasOwn(value, key)
        ? value[key]
        : undefined;
    if (next === undefined) {
      throw new InputError(
        `entry ${entry}: the JSON Pointer ${pointer} names no value in the source`,
      );
    }
    value = next;
  }
  return value;
}
