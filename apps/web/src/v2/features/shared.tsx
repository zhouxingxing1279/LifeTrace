import type { Dispatch, ReactNode, SetStateAction } from "react";
import type { LifeTraceState } from "../model";

export type SetLifeTraceState = Dispatch<SetStateAction<LifeTraceState>>;

export function PageHeader({ title, detail, action }: { title: string; detail: string; action?: ReactNode }) {
  return <header className="lt-page-header"><div className="lt-row-between"><div><h1>{title}</h1><p>{detail}</p></div>{action}</div></header>;
}

export function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return <section className="lt-section"><div className="lt-section-header"><h2>{title}</h2>{action}</div>{children}</section>;
}
