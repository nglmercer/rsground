// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from "vitest";

import { createLocalStoredSignal } from "./createLocalStoredSignal";

const values = new Map<string, string>();
const storage = {
  getItem: (key: string) => values.get(key) ?? null,
  setItem: (key: string, value: string) => values.set(key, value),
  removeItem: (key: string) => values.delete(key),
  clear: () => values.clear(),
};

describe("createLocalStoredSignal", () => {
  beforeEach(() => {
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      value: storage,
    });
    storage.clear();
    vi.restoreAllMocks();
  });

  it("restores stored values and persists signal updates", async () => {
    window.localStorage.setItem("counter", "7");

    const [value, setValue] = createLocalStoredSignal<number | null>(
      "counter",
      0,
      Number,
      String,
    );

    expect(value()).toBe(7);
    setValue(8);
    expect(value()).toBe(8);
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(window.localStorage.getItem("counter")).toBe("8");

    setValue(null);
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(window.localStorage.getItem("counter")).toBeNull();
  });

  it("falls back to the default when deserialization fails", () => {
    window.localStorage.setItem("value", "invalid");
    const error = vi.spyOn(console, "error").mockImplementation(() => {});

    const [value] = createLocalStoredSignal(
      "value",
      "default",
      (stored) => JSON.parse(stored),
      JSON.stringify,
    );

    expect(value()).toBe("default");
    expect(error).toHaveBeenCalledOnce();
  });
});
