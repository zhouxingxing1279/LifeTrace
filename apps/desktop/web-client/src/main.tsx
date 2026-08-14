import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import ClientErrorBoundary from "../../src/components/ClientErrorBoundary";
import { installGlobalErrorHandlers } from "../../src/services/clientObservability";
import { installGlobalFetchInstrumentation } from "../../src/services/fetchInstrumentation";
import "./bootstrap";
import App from "./App";

/* Existing feature detail styles first; the new browser system is authoritative. */
import "./styles.css";
import "./cloud-pages.css";
import "./web-tokens.css";
import "./web-primitives.css";
import "./web-shell.css";
import "./web-auth.css";
import "./web-workspaces.css";
import "./web-beecount.css";
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
