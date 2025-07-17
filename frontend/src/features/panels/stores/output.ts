import { createStore } from "solid-js/store";

export const [outputPanel, setOutputPanel] = createStore<Array<string>>([]);
