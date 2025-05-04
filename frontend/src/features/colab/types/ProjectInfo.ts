import { AccessLevel } from "@features/ws/types";

export interface ProjectInfo {
  id: string,
  name?: string,
  owner?: string,
  users: Record<string, [string, AccessLevel]>,
  requests: Record<string, string>,
  is_public: boolean,
  password?: string,
}
