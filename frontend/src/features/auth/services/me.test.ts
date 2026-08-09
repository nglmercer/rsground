import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@services", () => ({
  BACKEND_HOST: "http://backend.test",
}));

import { ApiPath, HttpHeader, HttpMethod } from "@constants";
import { fetchMe } from "./me";

describe("fetchMe", () => {
  const fetchMock = vi.fn<typeof fetch>();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("fetches the current user with a bearer token", async () => {
    const userInfo = {
      id: "user-id",
      is_guest: true,
      name: "Ada",
      exp: 1_999_999_999,
    };
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify(userInfo), { status: 200 }),
    );

    await expect(fetchMe("jwt")).resolves.toEqual(userInfo);
    expect(fetchMock).toHaveBeenCalledWith(
      `http://backend.test${ApiPath.AuthMe}`,
      {
        method: HttpMethod.Get,
        headers: {
          [HttpHeader.Authorization]: "Bearer jwt",
        },
      },
    );
  });

  it("returns null for an expired or invalid session", async () => {
    fetchMock.mockResolvedValue(new Response("expired", { status: 401 }));

    await expect(fetchMe("expired-jwt")).resolves.toBeNull();
  });

  it("surfaces server failures instead of treating them as an expired session", async () => {
    fetchMock.mockResolvedValue(new Response("unavailable", { status: 503 }));

    await expect(fetchMe("jwt")).rejects.toThrow(
      "Unable to verify the session (503)",
    );
  });
});
