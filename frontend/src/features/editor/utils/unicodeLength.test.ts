import { describe, expect, it } from "vitest";

import { unicodeLength } from "./unicodeLength";

describe("unicodeLength", () => {
  it("counts Unicode code points instead of UTF-16 code units", () => {
    expect(unicodeLength("")).toBe(0);
    expect(unicodeLength("hello")).toBe(5);
    expect(unicodeLength("🌎🙂")).toBe(2);
    expect(unicodeLength("e\u0301")).toBe(2);
  });
});
