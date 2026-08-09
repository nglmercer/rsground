import { describe, expect, it } from "vitest";

import { sameValueRecord } from "./sameValueRecord";

describe("sameValueRecord", () => {
  it("assigns the same value to every requested key", () => {
    const value = { enabled: true };
    const record = sameValueRecord(["first", "second"], value);

    expect(record).toEqual({ first: value, second: value });
    expect(record.first).toBe(value);
    expect(record.second).toBe(value);
  });

  it("supports numeric keys and empty key lists", () => {
    expect(sameValueRecord([1, 2], "shared")).toEqual({
      1: "shared",
      2: "shared",
    });
    expect(sameValueRecord([], 42)).toEqual({});
  });
});
