import "./app.css";
import App from "./App.svelte";
import { mount } from "svelte";

function requireDesktopRoot(): HTMLElement {
  const target = document.getElementById("app");
  if (!target) {
    throw new Error("CanISend desktop root element is missing");
  }
  return target;
}

const target = requireDesktopRoot();

function boundedErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return message.slice(0, 500);
}

function renderStartupFailure(error: unknown): void {
  target.replaceChildren();
  const panel = document.createElement("main");
  panel.setAttribute("role", "alert");
  panel.setAttribute("aria-label", "CanISend startup failure");
  panel.style.cssText =
    "max-width:48rem;margin:4rem auto;padding:2rem;font:16px/1.6 system-ui;color:#7f1d1d";
  const heading = document.createElement("h1");
  heading.textContent = "CanISend could not start";
  const detail = document.createElement("p");
  detail.textContent = boundedErrorMessage(error);
  panel.append(heading, detail);
  target.append(panel);
}

window.addEventListener("error", (event) => renderStartupFailure(event.error ?? event.message));
window.addEventListener("unhandledrejection", (event) => renderStartupFailure(event.reason));

async function mountRoot(): Promise<void> {
  const galleryRequested =
    import.meta.env.DEV && new URLSearchParams(window.location.search).get("ui-system") === "1";
  const Root = galleryRequested
    ? (await import("$lib/components/patterns/UiSystemGallery.svelte")).default
    : App;
  mount(Root, { target });
}

void mountRoot().catch(renderStartupFailure);
