import { describe, expect, it } from "vitest";

import { applyOperation, applyOperationToString } from "./applyOperation";

type OperationLike = Parameters<typeof applyOperation>[0];

function operation(operations: unknown[], noop = false): OperationLike {
  return {
    is_noop: () => noop,
    to_string: () => JSON.stringify(operations),
  } as OperationLike;
}

describe("applyOperation", () => {
  it("does nothing for a no-op operation", () => {
    const insertions: unknown[] = [];
    const deletions: unknown[] = [];

    expect(
      applyOperation(
        operation([], true),
        (...args) => insertions.push(args),
        (...args) => deletions.push(args),
      ),
    ).toBe(false);
    expect(insertions).toEqual([]);
    expect(deletions).toEqual([]);
  });

  it("converts serialized retain, insert, and delete actions to editor ranges", () => {
    const insertions: unknown[] = [];
    const deletions: unknown[] = [];

    expect(
      applyOperation(
        operation([5, " brave", -6]),
        (...args) => insertions.push(args),
        (...args) => deletions.push(args),
      ),
    ).toBe(true);

    expect(insertions).toEqual([[5, " brave"]]);
    expect(deletions).toEqual([[11, 17]]);
  });

  it("applies Unicode-aware operations to strings", () => {
    expect(
      applyOperationToString(
        "hello world",
        operation([5, " 🌎", 6]),
      ),
    ).toBe("hello 🌎 world");

    expect(
      applyOperationToString(
        "hello world",
        operation([5, -6]),
      ),
    ).toBe("hello");
  });
});
