import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { LifeTraceApp } from "../../web/src/v2/App";
import "../../web/src/v2/v2.css";
import { desktopPlatform } from "../src/platform-v2/desktop";

const root = document.getElementById("root");
if (!root) throw new Error("LifeTrace Desktop root element was not found");

createRoot(root).render(<StrictMode><LifeTraceApp platform={desktopPlatform} /></StrictMode>);
