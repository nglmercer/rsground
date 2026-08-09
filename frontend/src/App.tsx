import "./App.sass";

import {
  Component,
  createSignal,
  lazy,
  Match,
  onMount,
  Suspense,
  Switch,
} from "solid-js";
import { AppStatus, AppStatusProps } from "@components/AppStatus";
import { checkForAuth, interceptAuthCallback } from "@features/auth/utils";
import { interceptProjectRoutes } from "@features/colab/utils";

import "@features/theme/stores"

const Sidebar = lazy(() =>
  import("@features/sidebar/views").then(({ Sidebar }) => ({ default: Sidebar }))
);

const Panels = lazy(() =>
  import("@features/panels/views").then(({ Panels }) => ({ default: Panels }))
);

type InitializationStatus =
  | AppStatusProps
  | {
    kind: "ready";
  };

const App: Component = () => {
  const [status, setStatus] = createSignal<InitializationStatus>({
    kind: "loading",
    title: "Preparing your workspace",
    message: "Checking your session…",
  });
  let initializationId = 0;

  const initialize = () => {
    const id = ++initializationId;
    void initializeApp((nextStatus) => {
      if (id === initializationId) setStatus(nextStatus);
    });
  };

  onMount(() => {
    initialize();
  });

  return <Switch>
    <Match when={status().kind === "ready"}>
      <Suspense
        fallback={
          <AppStatus
            kind="loading"
            title="Loading the editor"
            message="Almost there…"
          />
        }
      >
        <Sidebar />
        <Panels />
      </Suspense>
    </Match>

    <Match when={status().kind !== "ready"}>
      {(() => {
        const current = status() as AppStatusProps;
        return (
          <AppStatus
            kind={current.kind}
            title={current.title}
            message={current.message}
            action={current.action}
          />
        );
      })()}
    </Match>
  </Switch>;
};

async function initializeApp(
  setStatus: (status: InitializationStatus) => void,
): Promise<void> {
  try {
    setStatus({
      kind: "loading",
      title: "Signing you in",
      message: "Checking your session…",
    });

    await interceptAuthCallback();

    if ((await checkForAuth()) === "redirecting") {
      setStatus({
        kind: "redirecting",
        title: "Redirecting to GitHub",
        message: "You’ll be sent back to your workspace after sign-in.",
      });
      return;
    }

    // Keep the sync listener out of the initial bundle with the editor UI.
    const { startReceivingSync } = await import("@features/editor/services");
    startReceivingSync();

    setStatus({
      kind: "loading",
      title: "Opening your project",
      message: "Loading files and collaboration settings…",
    });
    if ((await interceptProjectRoutes()) === "redirecting") {
      setStatus({
        kind: "redirecting",
        title: "Opening your project",
        message: "Redirecting to the new workspace…",
      });
      return;
    }

    setStatus({ kind: "ready" });
  } catch (error) {
    console.error("Unable to initialize RsGround:", error);
    setStatus({
      kind: "error",
      title: "RsGround could not start",
      message: getInitializationErrorMessage(error),
      action: {
        label: "Try again",
        onClick: () => window.location.reload(),
      },
    });
  }
}

function getInitializationErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) return error.message;

  return "We couldn’t load your workspace. Check your connection and try again.";
}

export default App;
