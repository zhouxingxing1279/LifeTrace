import { useState } from "react";
import {
  Bell,
  CheckCircle2,
  Info,
  Plus,
  Trash2,
} from "lucide-react";
import {
  Badge,
  Button,
  Checkbox,
  Field,
  IconButton,
  Input,
  Kbd,
  SearchInput,
  Select,
  Skeleton,
  Spinner,
  Switch,
  Tabs,
  Textarea,
  Tooltip,
} from "@/src/components/ui";
import {
  EmptyState,
  ErrorState,
  LoadingState,
  PanelHead,
  StatDisplay,
} from "@/src/components/common";
import { PageContainer, Section, Toolbar } from "@/src/components/layout";
import { useLifeStore } from "@/src/stores/useLifeStore";

function DemoRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="gallery-row">
      <span>{label}</span>
      <div>{children}</div>
    </div>
  );
}

/** 设计系统演示页：等价于 Storybook 的组件画廊，供开发与验收使用。 */
export default function DesignGallery() {
  const dark = useLifeStore((state) => state.dark);
  const toggleDark = useLifeStore((state) => state.toggleDark);
  const [switchOn, setSwitchOn] = useState(true);
  const [checked, setChecked] = useState(false);
  const [tab, setTab] = useState<"preview" | "code">("preview");
  const [search, setSearch] = useState("");

  return (
    <PageContainer>
      <Section
        title="Design System"
        description="LifeTrace 统一组件画廊 —— 新页面开发前先在这里确认组件与视觉规范。"
        actions={
          <Button
            variant="secondary"
            icon={<Info aria-hidden="true" />}
            onClick={toggleDark}
          >
            {dark ? "切换浅色主题" : "切换深色主题"}
          </Button>
        }
      >
        <div className="gallery-grid">
          <article className="hx-panel">
            <PanelHead kicker="Buttons" title="按钮" />
            <div className="hx-panel-body gallery-body">
              <DemoRow label="变体">
                <Button variant="primary">主要操作</Button>
                <Button variant="secondary">次要操作</Button>
                <Button variant="ghost">幽灵按钮</Button>
                <Button variant="danger" icon={<Trash2 aria-hidden="true" />}>
                  危险操作
                </Button>
              </DemoRow>
              <DemoRow label="尺寸与状态">
                <Button variant="primary" size="sm">
                  小
                </Button>
                <Button variant="primary" loading>
                  加载中
                </Button>
                <Button variant="secondary" disabled>
                  已禁用
                </Button>
                <IconButton label="示例图标按钮">
                  <Bell aria-hidden="true" />
                </IconButton>
              </DemoRow>
            </div>
          </article>

          <article className="hx-panel">
            <PanelHead kicker="Form" title="表单控件" />
            <div className="hx-panel-body gallery-body">
              <DemoRow label="输入">
                <Field label="名称">
                  <Input placeholder="输入内容" />
                </Field>
                <Field label="类型">
                  <Select defaultValue="a">
                    <option value="a">选项 A</option>
                    <option value="b">选项 B</option>
                  </Select>
                </Field>
              </DemoRow>
              <DemoRow label="文本域">
                <Textarea placeholder="多行文本…" />
              </DemoRow>
              <DemoRow label="搜索">
                <SearchInput value={search} onChange={setSearch} placeholder="搜索" />
              </DemoRow>
              <DemoRow label="开关与复选">
                <Switch checked={switchOn} onChange={setSwitchOn} label="示例开关" />
                <Checkbox checked={checked} onChange={setChecked} label="示例复选" />
              </DemoRow>
            </div>
          </article>

          <article className="hx-panel">
            <PanelHead kicker="Feedback" title="反馈状态" />
            <div className="hx-panel-body gallery-body">
              <DemoRow label="徽标">
                <Badge tone="neutral">默认</Badge>
                <Badge tone="success" dot>
                  已同步
                </Badge>
                <Badge tone="warning">待处理</Badge>
                <Badge tone="danger">失败</Badge>
                <Badge tone="info">信息</Badge>
                <Badge tone="primary">进行中</Badge>
              </DemoRow>
              <DemoRow label="键盘提示">
                <Kbd>Ctrl</Kbd> <Kbd>K</Kbd>
              </DemoRow>
              <DemoRow label="加载">
                <Spinner label="加载中" />
                <div className="gallery-skeleton">
                  <Skeleton width={120} height={12} />
                  <Skeleton width={200} height={12} />
                  <Skeleton width={160} height={12} />
                </div>
              </DemoRow>
              <DemoRow label="空状态">
                <EmptyState title="暂无数据" hint="创建第一条记录后显示在这里。" />
              </DemoRow>
              <DemoRow label="错误状态">
                <ErrorState
                  title="同步失败"
                  message="无法连接服务器，请检查网络后重试。"
                  onRetry={() => undefined}
                />
              </DemoRow>
            </div>
          </article>

          <article className="hx-panel">
            <PanelHead kicker="Layout" title="布局与信息" />
            <div className="hx-panel-body gallery-body">
              <DemoRow label="统计">
                <StatDisplay label="本月支出" value="¥1,234.00" sub="12 笔记录" />
                <StatDisplay label="总资产" value="¥8,760.00" sub="3 个账户" tone="positive" />
              </DemoRow>
              <DemoRow label="标签页">
                <Tabs
                  items={[
                    { value: "preview", label: "预览" },
                    { value: "code", label: "代码" },
                  ]}
                  value={tab}
                  onChange={setTab}
                />
              </DemoRow>
              <DemoRow label="工具栏">
                <Toolbar
                  left={<SearchInput value="" onChange={() => undefined} placeholder="筛选" />}
                  right={<Button variant="primary" icon={<Plus aria-hidden="true" />}>新建</Button>}
                />
              </DemoRow>
              <DemoRow label="提示">
                <Tooltip label="鼠标悬停显示说明">
                  <Button variant="ghost" icon={<CheckCircle2 aria-hidden="true" />}>
                    悬停查看
                  </Button>
                </Tooltip>
              </DemoRow>
            </div>
          </article>

          <article className="hx-panel">
            <PanelHead kicker="Loading" title="骨架屏" />
            <div className="hx-panel-body">
              <LoadingState rows={4} />
            </div>
          </article>

          <article className="hx-panel">
            <PanelHead kicker="Empty" title="空数据" />
            <div className="hx-panel-body">
              <EmptyState
                title="还没有坚持项目"
                hint="设定一个每天或每周的小目标，从今天开始记录。"
                icon={<CheckCircle2 aria-hidden="true" />}
                action={
                  <Button variant="primary" icon={<Plus aria-hidden="true" />}>
                    新建项目
                  </Button>
                }
              />
            </div>
          </article>
        </div>
      </Section>
    </PageContainer>
  );
}
