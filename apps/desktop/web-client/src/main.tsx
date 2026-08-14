import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import ClientErrorBoundary from "../../src/components/ClientErrorBoundary";
import { installGlobalErrorHandlers } from "../../src/services/clientObservability";
import { installGlobalFetchInstrumentation } from "../../src/services/fetchInstrumentation";
import "./bootstrap";
import App from "./App";
import "./styles.css";
import "./cloud-pages.css";
import "./web-shell.css";
import "./browser.css";

installGlobalFetchInstrumentation();
installGlobalErrorHandlers();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ClientErrorBoundary>
      <App />
    </ClientErrorBoundary>
  </StrictMode>,
);
