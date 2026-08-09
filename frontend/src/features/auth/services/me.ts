import { BACKEND_HOST } from "@services";
import { AuthInfo } from "../types";
import { ApiPath, HttpHeader, HttpMethod, HttpStatus } from "@constants";

export async function fetchMe(jwt: string): Promise<AuthInfo> {
  const res = await fetch(
    `${BACKEND_HOST}${ApiPath.AuthMe}`,
    {
      method: HttpMethod.Get,
      headers: {
        [HttpHeader.Authorization]: `Bearer ${jwt}`,
      },
    },
  );

  if (res.ok) {
    return await res.json();
  }

  if (res.status === HttpStatus.Unauthorized) {
    return null;
  }

  throw new Error(`Unable to verify the session (${res.status})`);
}
