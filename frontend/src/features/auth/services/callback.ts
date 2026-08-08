import { BACKEND_HOST } from "@services";
import { AuthInfo } from "../types/AuthInfo";

export async function authCallback(code: string): Promise<AuthInfo> {
  const res = await fetch(
    `${BACKEND_HOST}/auth/callback?code=${encodeURIComponent(code)}`,
  );

  if (!res.ok) {
    throw new Error(`GitHub login failed (${res.status})`);
  }

  return await res.json();
}
