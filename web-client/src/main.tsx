import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./bootstrap";
import App from "./App";
import "./styles.css";
import "./epic13.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
