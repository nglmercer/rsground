import { afterEach, describe, expect, it, vi } from "vitest";

import { generateRandomName } from "./random_name";

describe("generateRandomName", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("combines deterministic adjective, subject, and number choices", () => {
    vi.spyOn(Math, "random")
      .mockReturnValueOnce(0)
      .mockReturnValueOnce(0.999999)
      .mockReturnValueOnce(0.999999);

    expect(generateRandomName()).toBe("OxidizedUser998");
  });
});
