import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./bootstrap";
import App from "./App";
import "../../app/globals.css";
import "../../app/hengxu.css";
import "../../app/fitness-app.css";
import "../../app/english.css";
import "../../app/xunji-import.css";
import "../../app/notes.css";
import "../../app/redesign.css";
import "../../app/persist-project.css";
import "../../app/settings.css";
import "./styles.css";
import "./epic13.css";
import "./browser.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
