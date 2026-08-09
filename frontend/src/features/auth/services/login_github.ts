import { BACKEND_HOST } from "@services";
import { saveRedirectUrl } from "../stores";
import { ApiPath } from "@constants";

export function loginGithub(): void {
  saveRedirectUrl();

  window.location.href = `${BACKEND_HOST}${ApiPath.Auth}`;
}
