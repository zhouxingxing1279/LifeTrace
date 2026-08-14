import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import ClientErrorBoundary from "../../src/components/ClientErrorBoundary";
import { installGlobalErrorHandlers } from "../../src/services/clientObservability";
import { installGlobalFetchInstrumentation } from "../../src/services/fetchInstrumentation";
import "./bootstrap";
import App from "./App";

/* Legacy feature compatibility first; authoritative browser layers follow. */
import "./styles.css";
import "./cloud-pages.css";
import "./browser.css";
import "./web-tokens.css";
import "./web-primitives.css";
import "./web-shell.css";
import "./web-features.css";

installGlobalFetchInstrumentation();
installGlobalErrorHandlers();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ClientErrorBoundary>
      <App />
    </ClientErrorBoundary>
  </StrictMode>,
);
