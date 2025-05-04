import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";

import { AccessLevel } from "@features/ws/types";

import { ProjectInfo } from "../types";

export const [projectAccess, setProjectAccess] = createSignal<AccessLevel>(AccessLevel.Queue);

export const [isProjectOwner, setIsProjectOwner] = createSignal<boolean>(false);

export const [projectInfo, setProjectInfo] = createStore<ProjectInfo>({
  id: "",
  users: {},
  requests: {},
  is_public: false,
});
