import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { LifeTraceApp } from "./v2/App";
import { webPlatform } from "./v2/platform";
import "./v2/v2.css";

const root = document.getElementById("root");
if (!root) throw new Error("LifeTrace root element was not found");

createRoot(root).render(<StrictMode><LifeTraceApp platform={webPlatform} /></StrictMode>);
