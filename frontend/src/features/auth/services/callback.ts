import { BACKEND_HOST } from "@services";
import { AuthInfo } from "../types/AuthInfo";

export async function authCallback(code: string, state: string): Promise<AuthInfo> {
  const res = await fetch(
    `${BACKEND_HOST}/auth/callback?code=${encodeURIComponent(code)}&state=${encodeURIComponent(state)}`,
    { credentials: "include" },
  );

  if (!res.ok) {
    throw new Error(`GitHub login failed (${res.status})`);
  }

  return await res.json();
}
