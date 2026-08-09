import { BACKEND_HOST } from "@services";
import { AuthInfo } from "../types";
import { setAuthInfo } from "../stores";
import { ApiPath, HttpHeader, HttpMethod, HttpValue } from "@constants";

export async function loginGuest(guest_name: string): Promise<AuthInfo> {
  const res = await fetch(
    `${BACKEND_HOST}${ApiPath.AuthGuest}`,
    {
      method: HttpMethod.Post,
      headers: {
        [HttpHeader.ContentType]: HttpValue.JsonContentType,
      },
      body: JSON.stringify({
        guest_name,
      }),
    },
  );

  if (!res.ok) {
    throw new Error(`Guest login failed (${res.status})`);
  }

  const authInfo = await res.json();

  setAuthInfo(authInfo);

  return authInfo;
}
