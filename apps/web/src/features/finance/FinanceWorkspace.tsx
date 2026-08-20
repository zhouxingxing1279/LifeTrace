/*
 * LifeTrace finance is intentionally a thin mount point.
 *
 * The actual finance presentation is the source-derived BeeCount Cloud Web port
 * under ./beecount-cloud. Do not rebuild finance screens with LifeTrace generic
 * UI components here: LifeTrace owns only the outer AppShell/session.
 */
export { BeeCountCloudWorkspace as FinanceWorkspace } from "./beecount-cloud/BeeCountCloudWorkspace";
