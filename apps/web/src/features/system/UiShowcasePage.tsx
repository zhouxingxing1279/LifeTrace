import { useState } from "react";
import { Area, AreaChart, ResponsiveContainer, XAxis } from "recharts";
import {
  AlertDialog,
  Badge,
  Button,
  Card,
  CardContent,
  Checkbox,
  Dialog,
  EmptyState,
  Input,
  MetricCard,
  PageHeader,
  Progress,
  ScrollArea,
  Select,
  Separator,
  Sheet,
  Skeleton,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Tabs,
  Textarea,
  Toast,
} from "../../components/ui";

const chart = [{ day: "一", value: 2 }, { day: "二", value: 4 }, { day: "三", value: 3 }, { day: "四", value: 6 }, { day: "五", value: 5 }, { day: "六", value: 8 }, { day: "日", value: 7 }];

export function UiShowcasePage() {
  const [tab, setTab] = useState<"base" | "overlay">("base");
  const [enabled, setEnabled] = useState(true);
  const [dialog, setDialog] = useState(false);
  const [sheet, setSheet] = useState(false);
  const [alert, setAlert] = useState(false);

  return <div className="page-shell">
    <PageHeader title="UI Showcase" description="Phase 1 视觉基线：Button、Form、Table、Dialog、Sheet、Navigation、Metric、Chart、Loading 与 Feedback。" />
    <Tabs value={tab} onValueChange={setTab} items={[{ value: "base", label: "基础组件" }, { value: "overlay", label: "Overlay / Feedback" }]} />

    {tab === "base" ? <div className="mt-5 space-y-5">
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4"><MetricCard label="Metric" value="72%" hint="语义 token" /><MetricCard label="Income" value="¥8,240" hint="income color" /><MetricCard label="Expense" value="¥3,120" hint="expense color" /><MetricCard label="Trend" value="+12%" hint="Tremor-style" /></div>
      <div className="grid gap-5 xl:grid-cols-2">
        <Card><CardContent className="space-y-4 pt-5"><div className="font-semibold">Forms</div><Input placeholder="Input" /><Textarea placeholder="Textarea" /><Select defaultValue="one"><option value="one">Select option one</option><option value="two">Select option two</option></Select><div className="flex flex-wrap items-center gap-5"><label className="flex items-center gap-2 text-sm"><Checkbox defaultChecked />Checkbox</label><div className="flex items-center gap-2 text-sm"><Switch label="Showcase switch" checked={enabled} onCheckedChange={setEnabled} />Switch</div></div><div className="flex flex-wrap gap-2"><Button>Primary</Button><Button variant="secondary">Secondary</Button><Button variant="outline">Outline</Button><Button variant="ghost">Ghost</Button><Button variant="destructive">Destructive</Button></div><Progress value={72} /></CardContent></Card>
        <Card><CardContent className="pt-5"><div className="font-semibold">Chart</div><div className="mt-4 h-56"><ResponsiveContainer width="100%" height="100%"><AreaChart data={chart}><XAxis dataKey="day" tickLine={false} axisLine={false} /><Area type="monotone" dataKey="value" stroke="hsl(var(--chart-1))" fill="hsl(var(--chart-1))" fillOpacity={0.15} /></AreaChart></ResponsiveContainer></div></CardContent></Card>
      </div>
      <Card><CardContent className="pt-5"><div className="mb-3 font-semibold">Table</div><Table><TableHeader><TableRow><TableHead>组件</TableHead><TableHead>状态</TableHead><TableHead>说明</TableHead></TableRow></TableHeader><TableBody><TableRow><TableCell>Design Token</TableCell><TableCell><Badge className="text-success">PASS</Badge></TableCell><TableCell>Light / Dark semantic colors</TableCell></TableRow><TableRow><TableCell>Responsive</TableCell><TableCell><Badge>Matrix</Badge></TableCell><TableCell>Mobile → Wide</TableCell></TableRow></TableBody></Table></CardContent></Card>
    </div> : <div className="mt-5 grid gap-5 lg:grid-cols-2">
      <Card><CardContent className="space-y-3 pt-5"><div className="font-semibold">Overlay</div><div className="flex flex-wrap gap-2"><Button onClick={() => setDialog(true)}>Dialog</Button><Button variant="outline" onClick={() => setSheet(true)}>Sheet</Button><Button variant="destructive" onClick={() => setAlert(true)}>AlertDialog</Button></div><Separator /><Toast title="Toast / status" description="反馈组件视觉基线" tone="success" /><Dialog open={dialog} onOpenChange={setDialog} title="Dialog" description="Desktop overlay baseline"><Button onClick={() => setDialog(false)}>完成</Button></Dialog><Sheet open={sheet} onOpenChange={setSheet} title="Sheet"><EmptyState title="Sheet content" /></Sheet><AlertDialog open={alert} onOpenChange={setAlert} title="确认危险操作" description="AlertDialog baseline"><div className="flex justify-end gap-2"><Button variant="ghost" onClick={() => setAlert(false)}>取消</Button><Button variant="destructive" onClick={() => setAlert(false)}>确认</Button></div></AlertDialog></CardContent></Card>
      <Card><CardContent className="pt-5"><div className="font-semibold">Loading / Scroll</div><div className="mt-4 space-y-2"><Skeleton className="h-8 w-2/3" /><Skeleton className="h-4 w-full" /><Skeleton className="h-4 w-4/5" /></div><ScrollArea className="mt-5 h-32 rounded-md border p-3"><div className="space-y-3 text-sm text-muted-foreground">{Array.from({ length: 10 }, (_, index) => <p key={index}>Scrollable row {index + 1}</p>)}</div></ScrollArea></CardContent></Card>
    </div>}
  </div>;
}
