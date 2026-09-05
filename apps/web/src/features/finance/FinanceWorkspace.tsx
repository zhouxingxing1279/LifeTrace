/*
 * LifeTrace finance is intentionally a thin mount point.
 *
 * The actual finance presentation is the source-derived BeeCount Cloud Web port
 * under ./beecount-cloud. Do not rebuild finance screens with LifeTrace generic
 * UI components here: LifeTrace owns only the outer AppShell/session.
 *
 * The visually-hidden heading/source marker are compatibility semantics for the
 * global LifeTrace route/accessibility contract; they do not own finance UI.
 */
import { useApp } from "../../app/AppContext";
import { BeeCountCloudWorkspace } from "./beecount-cloud/BeeCountCloudWorkspace";

export function FinanceWorkspace() {
  const { session } = useApp();
  const financeSessionKey = session
    ? `${session.user.id}:${session.session.id}`
    : "anonymous";

  return (
    <>
      <h1 className="sr-only">财务</h1>
      <span className="sr-only">唯一财务数据源</span>
      <BeeCountCloudWorkspace key={financeSessionKey} />
    </>
  );
}
