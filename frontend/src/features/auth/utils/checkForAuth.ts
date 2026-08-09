import { authInfo } from "../stores";
import { fetchMe, loginGithub, loginGuest } from "../services";
import { generateRandomName } from "./random_name";

export type AuthCheckResult = "ready" | "redirecting";

export async function checkForAuth(): Promise<AuthCheckResult> {
  const storedAuth = authInfo();
  const { jwt, name, is_guest } = storedAuth ?? {};

  if (!jwt) {
    await loginGuest(generateRandomName());
    return "ready";
  }

  const userInfo = await fetchMe(jwt);

  // `/auth/me` validates the token but intentionally does not return it.
  // Keep the locally stored session (including jwt and avatar_url) intact.
  if (userInfo != null) return "ready";

  if (is_guest) {
    await loginGuest(name);
    return "ready";
  } else {
    loginGithub();
    return "redirecting";
  }
}
