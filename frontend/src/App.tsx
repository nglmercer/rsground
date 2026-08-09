import "./App.sass";
import "../public/fonts/inter.css";

import { Component, lazy, onMount, Suspense } from "solid-js";
import { Spinner } from "@components/Spinner";
import { checkForAuth, interceptAuthCallback } from "@features/auth/utils";
import { interceptProjectRoutes } from "@features/colab/utils";

import "@features/theme/stores"

const Sidebar = lazy(() =>
  import("@features/sidebar/views").then(({ Sidebar }) => ({ default: Sidebar }))
);

const Panels = lazy(() =>
  import("@features/panels/views").then(({ Panels }) => ({ default: Panels }))
);

const App: Component = () => {
  onMount(() => {
    void initializeApp();
  });

  return (
    <Suspense fallback={<Spinner />}>
      <Sidebar />
      <Panels />
    </Suspense>
  );
};

async function initializeApp() {
  try {
    await interceptAuthCallback();
    await checkForAuth();

    // Keep the sync listener out of the initial bundle with the editor UI.
    const { startReceivingSync } = await import("@features/editor/services");
    startReceivingSync();

    interceptProjectRoutes();
  } catch (error) {
    console.error("Unable to initialize RsGround:", error);
  }
}

export default App;
