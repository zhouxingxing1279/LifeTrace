import { Bot, Cloud, Languages, Monitor, ShieldCheck, CircleHelp } from "lucide-react";
import CloudSyncSettingsPanel from "@/src/components/CloudSyncSettingsPanel";
import AccountSecurityPanel from "@/src/components/account/AccountSecurityPanel";

const sections = [
  ["settings-general", "常规与外观", Monitor],
  ["settings-sync", "数据与同步", Cloud],
  ["settings-ai", "AI 服务", Bot],
  ["settings-translation", "翻译", Languages],
  ["settings-security", "账户与安全", ShieldCheck],
  ["settings-about", "关于", CircleHelp],
] as const;

function jumpTo(id: string) {
  window.location.hash = id;
  document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
}

export default function CloudAccountPanel() {
  return <>
    <aside className="hx-settings-nav" aria-label="设置分类">
      <h2>设置</h2>
      <nav>{sections.map(([id, label, Icon]) => <button key={id} type="button" onClick={() => jumpTo(id)}><Icon /><span>{label}</span></button>)}</nav>
    </aside>
    <CloudSyncSettingsPanel />
    <AccountSecurityPanel />
  </>;
}
