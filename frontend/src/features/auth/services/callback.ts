import { BACKEND_HOST } from "@services";
import { AuthInfo } from "../types/AuthInfo";
import { ApiPath, HttpValue } from "@constants";

export async function authCallback(code: string, state: string): Promise<AuthInfo> {
  const res = await fetch(
    `${BACKEND_HOST}${ApiPath.AuthCallback}?code=${encodeURIComponent(code)}&state=${encodeURIComponent(state)}`,
    { credentials: HttpValue.IncludeCredentials },
  );

  if (!res.ok) {
    throw new Error(`GitHub login failed (${res.status})`);
  }

  return await res.json();
}
