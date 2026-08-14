import { navigate } from "../navigation";
import { PageStack, Toolbar, type CloudPageProps } from "../ui";
import { ExecutionPage } from "./ExecutionPage";

export function ExecutionHubPage(props: CloudPageProps) {
  return <PageStack>
    <Toolbar>
      <button className="hx-btn primary">执行工作台</button>
      <button className="hx-btn secondary" onClick={() => navigate("/execution/goals")}>目标</button>
      <button className="hx-btn secondary" onClick={() => navigate("/execution/control")}>等待 / 提醒 / 依赖</button>
    </Toolbar>
    <ExecutionPage {...props} />
  </PageStack>;
}
