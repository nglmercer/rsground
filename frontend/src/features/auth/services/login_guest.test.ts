import { beforeEach, describe, expect, it, vi } from "vitest";

const { setAuthInfo } = vi.hoisted(() => ({
  setAuthInfo: vi.fn(),
}));

vi.mock("@services", () => ({
  BACKEND_HOST: "http://backend.test",
}));
vi.mock("../stores", () => ({ setAuthInfo }));

import { ApiPath, HttpHeader, HttpMethod, HttpValue } from "@constants";
import { loginGuest } from "./login_guest";

describe("loginGuest", () => {
  const fetchMock = vi.fn<typeof fetch>();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal("fetch", fetchMock);
  });

  it("sends the guest name and stores the returned auth info", async () => {
    const authInfo = {
      jwt: "jwt",
      id: "user-id",
      is_guest: true,
      name: "Ada",
    };
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify(authInfo), { status: 200 }),
    );

    await expect(loginGuest("Ada")).resolves.toEqual(authInfo);
    expect(fetchMock).toHaveBeenCalledWith(
      `http://backend.test${ApiPath.AuthGuest}`,
      {
        method: HttpMethod.Post,
        headers: {
          [HttpHeader.ContentType]: HttpValue.JsonContentType,
        },
        body: JSON.stringify({ guest_name: "Ada" }),
      },
    );
    expect(setAuthInfo).toHaveBeenCalledWith(authInfo);
  });

  it("surfaces a failed guest login without updating auth state", async () => {
    fetchMock.mockResolvedValue(new Response("rejected", { status: 400 }));

    await expect(loginGuest("Ada")).rejects.toThrow("Guest login failed (400)");
    expect(setAuthInfo).not.toHaveBeenCalled();
  });
});
