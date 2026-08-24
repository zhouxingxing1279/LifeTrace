import { Construction } from "lucide-react";
import { Card, CardContent, EmptyState, PageHeader } from "../../components/ui";

export function WorkspacePlaceholder({ title, description, references }: { title: string; description: string; references: string }) {
  return <div className="page-shell">
    <PageHeader title={title} description={description} />
    <Card><CardContent className="pt-5"><EmptyState icon={<Construction size={24} />} title="正在迁移到新 Web 工作区" description="该页面已进入独立 apps/web 路由，业务视图将在本次重构中替换，不再回退到 legacy Web。" /></CardContent></Card>
    <div className="mt-3 text-xs text-muted-foreground">Reference mapping: {references}</div>
  </div>;
}
