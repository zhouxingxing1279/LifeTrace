import type { ReactNode } from "react";
import { DesktopRuntimeProvider } from "./DesktopRuntime";

export default function DesktopProviders({ children }: { children: ReactNode }) {
  return <DesktopRuntimeProvider>{children}</DesktopRuntimeProvider>;
}
