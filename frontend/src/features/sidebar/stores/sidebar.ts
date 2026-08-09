import { createSignal } from "solid-js";
import { UiValue } from "@constants";

// Close sidebar when it uses more than 1/3 of screen
const defaultOpened = window.innerWidth > UiValue.SidebarBreakpointPx;

export const [isSidebarOpen, setIsSidebarOpen] = createSignal(defaultOpened);
