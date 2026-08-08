import "./App.sass";
import "../public/fonts/inter.css";

import { Component, createSignal, onMount, Show } from "solid-js";
import { Spinner } from "@components/Spinner";
import { checkForAuth, interceptAuthCallback } from "@features/auth/utils";
import { interceptProjectRoutes } from "@features/colab/utils";
import { startReceivingSync } from "@features/editor/services";
import { Panels } from "@features/panels/views";
import { Sidebar } from "@features/sidebar/views";

import "@features/theme/stores"

const App: Component = () => {
  const [ready, setReady] = createSignal(false);

  onMount(async () => {
    try {
      await interceptAuthCallback();
      await checkForAuth();
      interceptProjectRoutes();
      startReceivingSync();
    } catch (error) {
      console.error("Unable to initialize RsGround:", error);
    } finally {
      setReady(true);
    }
  });

  return (
    <Show when={ready()} fallback={<Spinner />}>
      <Sidebar />
      <Panels />
    </Show>
  );
};

export default App;
