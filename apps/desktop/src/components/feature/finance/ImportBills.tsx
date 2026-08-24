import { useEffect, useRef, useState } from "react";
import { Dumbbell, FileUp, Trash2 } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { Transaction } from "@/src/types";
import MobileUploadControl from "@/src/components/common/MobileUploadControl";
import { EmptyState, PanelHead } from "@/src/components/common";
import { notify } from "@/src/ui/feedback/toastBus";
import { pad, transactionAmountText } from "@/src/utils/format";

type ImportUploadItem = {
  id: string;
  kind: "fitness" | "bill";
  filename: string;
  contentType: string;
  size: number;
  status: "pending" | "parsed";
  createdAt: string;
};

type ImportRow = {
  type: Transaction["type"];
  amount: number;
  category: string;
  account: string;
  note?: string;
  occurredAt?: string;
  accountId?: string;
  toAccount?: string;
  toAccountId?: string;
  counterparty?: string;
  item?: string;
  sourceId?: string;
};

export default function ImportBills() {
  const { accounts, transactions, addTransaction } = useLifeStore();
  const input = useRef<HTMLInputElement>(null);
  const dragDepth = useRef(0);
  const [rows, setRows] = useState<ImportRow[]>([]);
  const [message, setMessage] = useState("");
  const [draggingBill, setDraggingBill] = useState(false);
  const [billSource, setBillSource] = useState<"wechat" | "alipay" | "generic">(
    "generic",
  );
  const [summary, setSummary] = useState({
    source: 0,
    neutral: 0,
    transfers: 0,
    unmatched: 0,
    duplicates: 0,
    invalid: 0,
  });
  const [importing, setImporting] = useState(false);
  const [phoneUploads, setPhoneUploads] = useState<ImportUploadItem[]>([]);
  const [loadingUploads, setLoadingUploads] = useState(true);

  const loadPhoneUploads = async () => {
    setLoadingUploads(true);
    try {
      const response = await fetch("/api/imports");
      const payload = (await response.json()) as { items?: ImportUploadItem[] };
      setPhoneUploads(payload.items ?? []);
    } finally {
      setLoadingUploads(false);
    }
  };

  useEffect(() => {
    const timer = window.setTimeout(() => void loadPhoneUploads(), 0);
    return () => window.clearTimeout(timer);
  }, []);

  const parseLine = (line: string) => {
    const result: string[] = [];
    let cell = "";
    let quoted = false;
    for (let i = 0; i < line.length; i++) {
      const c = line[i];
      if (c === '"' && line[i + 1] === '"') {
        cell += '"';
        i++;
      } else if (c === '"') {
        quoted = !quoted;
      } else if (c === "," && !quoted) {
        result.push(cell);
        cell = "";
      } else {
        cell += c;
      }
    }
    result.push(cell);
    return result;
  };

  const decodeCsv = async (file: File) => {
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
      return new TextDecoder("utf-8").decode(bytes.subarray(3));
    }
    try {
      return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      return new TextDecoder("gb18030").decode(bytes);
    }
  };

  const cellText = (value: unknown) =>
    value instanceof Date
      ? `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())} ${pad(value.getHours())}:${pad(value.getMinutes())}:${pad(value.getSeconds())}`
      : String(value ?? "").trim();

  const inferCategory = (
    type: "income" | "expense",
    transactionType: string,
    counterparty: string,
    item: string,
  ) => {
    const text = `${transactionType} ${counterparty} ${item}`;
    if (type === "income") return /退款|退还|退回/.test(text) ? "退款" : "其他收入";
    if (/餐饮|餐厅|饭|面|粉|水煮鱼|豆制品|咖啡|茶|奶茶|麦当劳|肯德基|食堂|亚惠/.test(text))
      return "餐饮";
    if (/地铁|公交|打车|滴滴|铁路|航空|加油|停车|充电/.test(text)) return "交通";
    if (/拼多多|淘宝|京东|商户消费|超市|便利店|眼镜|百货/.test(text)) return "购物";
    if (/医院|药房|诊所|医疗|体检/.test(text)) return "医疗健康";
    if (/话费|电费|水费|燃气|宽带|物业/.test(text)) return "生活缴费";
    if (/红包|群收款|转账|二维码付款|扫二维码/.test(text)) return "转账与人情";
    return "日常消费";
  };

  const read = async (file: File) => {
    try {
      setRows([]);
      setMessage("正在解析账单…");
      let matrix: unknown[][];
      if (file.name.toLowerCase().endsWith(".xlsx")) {
        const { readSheet } = await import("read-excel-file/browser");
        matrix = await readSheet(file);
      } else {
        const lines = (await decodeCsv(file))
          .replace(/^\uFEFF/, "")
          .split(/\r?\n/)
          .filter(Boolean);
        matrix = lines.map(parseLine);
      }
      const headerRow = matrix.findIndex((row) => {
        const text = row.map(cellText);
        return (
          text.some(
            (item) => item.includes("交易时间") || item.toLowerCase().includes("date"),
          ) &&
          text.some(
            (item) => item.includes("金额") || item.toLowerCase().includes("amount"),
          )
        );
      });
      if (headerRow < 0)
        throw new Error(
          "没有找到支付账单明细表头，请确认文件是微信或支付宝导出的 CSV / Excel 账单",
        );
      const headers = matrix[headerRow].map((value) =>
        cellText(value).replace(/\s/g, ""),
      );
      const source = headers.some(
        (header) => header.includes("收/付款方式") || header.includes("交易订单号"),
      )
        ? ("alipay" as const)
        : headers.some((header) => header.includes("微信"))
          ? ("wechat" as const)
          : ("generic" as const);
      setBillSource(source);
      const index = (...names: string[]) =>
        headers.findIndex((header) =>
          names.some((name) => header.toLowerCase().includes(name.toLowerCase())),
        );
      const dateIndex = index("交易时间", "时间", "日期", "date");
      const transactionTypeIndex = index("交易类型", "交易分类");
      const counterpartyIndex = index("交易对方", "交易对象", "商户", "counterparty");
      const itemIndex = index("商品", "说明", "item");
      const directionIndex = index("收/支", "收支", "direction");
      const amountIndex = index("金额", "amount");
      const accountIndex = index(
        "收/付款方式",
        "付款方式",
        "支付方式",
        "账户",
        "account",
      );
      const statusIndex = index("当前状态", "状态");
      const sourceIdIndex = index("交易订单号", "交易单号");
      const categoryIndex = index("分类", "category");
      if (dateIndex < 0 || amountIndex < 0 || directionIndex < 0)
        throw new Error("账单缺少交易时间、收/支或金额列");
      const existingIds = new Set(
        transactions.flatMap((item) => {
          const match = item.note?.match(/(?:微信|支付宝)交易单号：([^\s·]+)/);
          return match ? [match[1]] : [];
        }),
      );
      let neutral = 0;
      let transfers = 0;
      let unmatched = 0;
      let duplicates = 0;
      let invalid = 0;
      const parsed: ImportRow[] = [];
      const matchAccount = (rawAccount: string) => {
        const direct = accounts.find(
          (candidate) =>
            candidate.name === rawAccount ||
            rawAccount.includes(candidate.name) ||
            (candidate.last4 && rawAccount.includes(candidate.last4)),
        );
        if (direct) return direct;
        if (/银行|信用卡/.test(rawAccount)) {
          const namedBank = accounts.find(
            (candidate) =>
              candidate.type === "bank" &&
              candidate.name.replace(/^中国/, "") !== "银行" &&
              rawAccount.includes(candidate.name.replace(/^中国/, "")),
          );
          if (namedBank) return namedBank;
          const banks = accounts.filter((candidate) => candidate.type === "bank");
          return banks.length === 1 ? banks[0] : undefined;
        }
        if (/零钱|微信/.test(rawAccount))
          return accounts.find((candidate) => candidate.type === "wechat");
        if (/支付宝|余额宝|花呗|账户余额/.test(rawAccount))
          return accounts.find((candidate) => candidate.type === "alipay");
        return undefined;
      };
      for (const rawRow of matrix.slice(headerRow + 1)) {
        const cells = rawRow.map(cellText);
        if (cells.every((cell) => !cell)) continue;
        const direction = cells[directionIndex] ?? "";
        const transactionType = cells[transactionTypeIndex] ?? "";
        const sourceName =
          source === "alipay" ? "支付宝" : source === "wechat" ? "微信支付" : "支付账单";
        const counterparty = cells[counterpartyIndex] || sourceName;
        const item = (cells[itemIndex] ?? "").replace(/^\/$/, "");
        const neutralDirection = /中性|\/|不计收支/.test(direction);
        const isYield =
          neutralDirection && /收益发放|收益结转/.test(`${transactionType} ${counterparty} ${item}`);
        const isTransfer =
          neutralDirection &&
          /余额宝.*(?:转入|收款)|(?:转入|收款).*余额宝/.test(
            `${transactionType} ${counterparty} ${item}`,
          );
        if (neutralDirection && !isYield && !isTransfer) {
          neutral++;
          continue;
        }
        const type = isYield
          ? ("income" as const)
          : isTransfer
            ? ("transfer" as const)
            : /收入|income|入账/i.test(direction)
              ? ("income" as const)
              : /支出|expense/i.test(direction)
                ? ("expense" as const)
                : null;
        const amount = Math.abs(
          Number((cells[amountIndex] ?? "").replace(/[¥￥,\s]/g, "")),
        );
        const dateText = cells[dateIndex] ?? "";
        const parsedDate = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(dateText)
          ? new Date(`${dateText.replace(" ", "T")}+08:00`)
          : new Date(dateText);
        if (
          !type ||
          !Number.isFinite(amount) ||
          amount <= 0 ||
          !dateText ||
          Number.isNaN(parsedDate.getTime())
        ) {
          invalid++;
          continue;
        }
        const occurredAt = parsedDate.toISOString();
        const sourceId = (cells[sourceIdIndex] ?? "").trim();
        if (sourceId && existingIds.has(sourceId)) {
          duplicates++;
          continue;
        }
        const rawAccount = (cells[accountIndex] ?? "").replace(/^\/$/, "").trim();
        const account = matchAccount(rawAccount);
        const toAccount = isTransfer ? matchAccount("余额宝") : undefined;
        const status = (cells[statusIndex] ?? "").replace(/^\/$/, "");
        if (isTransfer) transfers++;
        const row: ImportRow = {
          type,
          amount,
          category: isTransfer
            ? "账户转账"
            : cells[categoryIndex] ||
              inferCategory(
                type === "income" ? "income" : "expense",
                transactionType,
                counterparty,
                item,
              ),
          account: account?.name || rawAccount || sourceName,
          accountId: account?.id,
          toAccount: isTransfer ? toAccount?.name || "余额宝" : undefined,
          toAccountId: toAccount?.id,
          counterparty,
          item,
          occurredAt,
          note: [
            transactionType,
            status,
            sourceId
              ? `${source === "alipay" ? "支付宝" : "微信"}交易单号：${sourceId}`
              : "",
          ]
            .filter(Boolean)
            .join(" · "),
          sourceId,
        };
        if (!row.accountId || (row.type === "transfer" && !row.toAccountId))
          unmatched++;
        parsed.push(row);
        if (sourceId) existingIds.add(sourceId);
      }
      setRows(parsed);
      setSummary({
        source: matrix.length - headerRow - 1,
        neutral,
        transfers,
        unmatched,
        duplicates,
        invalid,
      });
      setMessage(
        `已识别 ${parsed.length} 笔可导入记录${transfers ? `，其中 ${transfers} 笔账户转账` : ""}${unmatched ? `，${unmatched} 笔尚未匹配账户` : ""}${duplicates ? `，自动跳过 ${duplicates} 笔重复账单` : ""}`,
      );
    } catch (error) {
      setRows([]);
      setMessage(error instanceof Error ? error.message : "文件解析失败");
    } finally {
      if (input.current) input.current.value = "";
    }
  };

  const acceptBillFile = (file?: File) => {
    if (!file) return;
    if (!/\.(csv|xlsx)$/i.test(file.name)) {
      setRows([]);
      setMessage("仅支持 CSV 或 Excel（.xlsx）账单文件");
      return;
    }
    void read(file);
  };

  const parsePhoneBill = async (item: ImportUploadItem) => {
    setMessage(`正在从手机文件解析：${item.filename}`);
    const response = await fetch(`/api/imports?id=${encodeURIComponent(item.id)}`);
    if (!response.ok) {
      setMessage("无法读取手机上传的账单文件");
      return;
    }
    const file = new File([await response.blob()], item.filename, {
      type: item.contentType,
    });
    await read(file);
    await fetch("/api/imports", {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id: item.id, status: "parsed" }),
    });
    await loadPhoneUploads();
  };

  const deletePhoneUpload = async (id: string) => {
    await fetch(`/api/imports?id=${encodeURIComponent(id)}`, { method: "DELETE" });
    await loadPhoneUploads();
  };

  const commit = async () => {
    setImporting(true);
    try {
      for (const row of rows) {
        const transaction: Parameters<typeof addTransaction>[0] = {
          type: row.type,
          amount: row.amount,
          category: row.category,
          account: row.account,
          note: row.note,
          occurredAt: row.occurredAt,
          accountId: row.accountId,
          toAccount: row.toAccount,
          toAccountId: row.toAccountId,
          counterparty: row.counterparty,
          item: row.item,
        };
        await addTransaction(transaction);
      }
      const sourceName =
        billSource === "alipay" ? "支付宝" : billSource === "wechat" ? "微信" : "支付";
      setMessage(`已导入 ${rows.length} 笔${sourceName}账单到 SQLite`);
      setRows([]);
      notify(`${sourceName}账单导入完成`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "账单导入失败");
    } finally {
      setImporting(false);
    }
  };

  return (
    <div className="hx-view">
      <article className="hx-panel hx-phone-imports">
        <PanelHead kicker="手机传输" title="待处理导入文件" />
        <div className="hx-panel-body">
          <MobileUploadControl context="bills" />
          <div className="hx-phone-import-head">
            <p>手机上传的健身数据图和账单文件保存在电脑本地。</p>
            <button
              type="button"
              className="hx-btn secondary"
              onClick={() => void loadPhoneUploads()}
            >
              刷新列表
            </button>
          </div>
          <div className="hx-phone-import-list">
            {phoneUploads.map((item) => (
              <article key={item.id}>
                <span>
                  {item.kind === "fitness" ? <Dumbbell /> : <FileUp />}
                </span>
                <div>
                  <strong>{item.filename}</strong>
                  <small>
                    {item.kind === "fitness" ? "健身数据图" : "账单文件"} ·{" "}
                    {(item.size / 1024 / 1024).toFixed(2)} MB ·{" "}
                    {new Date(item.createdAt).toLocaleString("zh-CN")}
                  </small>
                </div>
                <div>
                  <a
                    className="hx-btn secondary"
                    href={`/api/imports?id=${encodeURIComponent(item.id)}`}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {item.kind === "fitness" ? "查看" : "下载"}
                  </a>
                  {item.kind === "bill" && /\.(xlsx|csv)$/i.test(item.filename) ? (
                    <button
                      type="button"
                      className="hx-btn primary"
                      onClick={() => void parsePhoneBill(item)}
                    >
                      {item.status === "parsed" ? "重新解析" : "电脑解析"}
                    </button>
                  ) : null}
                  <button
                    type="button"
                    className="hx-icon-btn"
                    aria-label={`删除${item.filename}`}
                    onClick={() => void deletePhoneUpload(item.id)}
                  >
                    <Trash2 />
                  </button>
                </div>
              </article>
            ))}
            {!phoneUploads.length && !loadingUploads ? (
              <EmptyState title="手机还没有上传文件" />
            ) : null}
            {loadingUploads ? (
              <EmptyState title="正在读取手机上传文件…" />
            ) : null}
          </div>
        </div>
      </article>

      <div className="hx-import-grid">
        <article className="hx-panel">
          <PanelHead kicker="微信 / 支付宝" title="导入支付流水" />
          <div className="hx-panel-body">
            <div
              className={`hx-drop${draggingBill ? " is-dragging" : ""}`}
              role="button"
              tabIndex={0}
              aria-label="拖放或选择微信、支付宝账单文件"
              onClick={() => input.current?.click()}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  input.current?.click();
                }
              }}
              onDragEnter={(event) => {
                event.preventDefault();
                dragDepth.current += 1;
                setDraggingBill(true);
              }}
              onDragOver={(event) => {
                event.preventDefault();
                event.dataTransfer.dropEffect = "copy";
              }}
              onDragLeave={(event) => {
                event.preventDefault();
                dragDepth.current = Math.max(0, dragDepth.current - 1);
                if (dragDepth.current === 0) setDraggingBill(false);
              }}
              onDrop={(event) => {
                event.preventDefault();
                dragDepth.current = 0;
                setDraggingBill(false);
                acceptBillFile(event.dataTransfer.files?.[0]);
              }}
            >
              <FileUp />
              <h3>{draggingBill ? "松开即可解析账单" : "拖动账单文件到这里"}</h3>
              <p>支持微信 Excel / CSV，以及支付宝导出的 GBK 或 UTF-8 CSV。</p>
              <input
                ref={input}
                type="file"
                hidden
                accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,.csv,text/csv"
                onChange={(event) => acceptBillFile(event.target.files?.[0])}
              />
              <button
                type="button"
                className="hx-btn primary"
                onClick={(event) => {
                  event.stopPropagation();
                  input.current?.click();
                }}
              >
                选择账单文件
              </button>
            </div>
            {message ? (
              <p className="hx-inline-message" role="status" aria-live="polite">
                {message}
              </p>
            ) : null}
            {rows.length > 0 ? (
              <div className="hx-import-preview">
                <div>
                  {rows.slice(0, 12).map((row, index) => (
                    <span key={`${row.sourceId ?? index}`}>
                      <b>{row.counterparty}</b>
                      <small>
                        {row.category} ·{" "}
                        {row.type === "transfer"
                          ? `${row.account} → ${row.toAccount ?? "未匹配账户"}`
                          : row.account}{" "}
                        · {new Date(row.occurredAt ?? "").toLocaleDateString("zh-CN")}
                        {!row.accountId ||
                        (row.type === "transfer" && !row.toAccountId)
                          ? " · 未匹配账户"
                          : ""}
                      </small>
                      <strong>{transactionAmountText(row)}</strong>
                    </span>
                  ))}
                </div>
                {rows.length > 12 ? (
                  <small>另有 {rows.length - 12} 笔记录将在确认后一起导入</small>
                ) : null}
                <button
                  type="button"
                  className="hx-btn primary"
                  disabled={importing}
                  onClick={commit}
                >
                  {importing ? "正在导入…" : `确认导入 ${rows.length} 笔`}
                </button>
              </div>
            ) : null}
          </div>
        </article>
        <aside className="hx-panel">
          <PanelHead kicker="识别结果" title="支付账单规则" />
          <div className="hx-panel-body hx-rules">
            <p>
              <b>1</b> 自动识别 UTF-8 / GBK 编码，并跳过文件顶部说明。
            </p>
            <p>
              <b>2</b> 余额宝收益按收入导入，账户间转入按转账保存，不虚增收支。
            </p>
            <p>
              <b>3</b> 分别使用微信、支付宝交易单号去重，避免重复入账。
            </p>
            <p>
              <b>4</b> 只有匹配到账户且晚于余额基准时间的流水，才会影响余额。
            </p>
            {summary.source > 0 ? (
              <div className="hx-import-stats">
                <span>
                  明细行 <b>{summary.source}</b>
                </span>
                <span>
                  账户转账 <b>{summary.transfers}</b>
                </span>
                <span>
                  未匹配账户 <b>{summary.unmatched}</b>
                </span>
                <span>
                  其他中性交易 <b>{summary.neutral}</b>
                </span>
                <span>
                  重复账单 <b>{summary.duplicates}</b>
                </span>
                <span>
                  无效记录 <b>{summary.invalid}</b>
                </span>
              </div>
            ) : null}
            <hr />
            <small>
              当前已有 {transactions.length} 笔账单，{accounts.length} 个账户。
            </small>
          </div>
        </aside>
      </div>
    </div>
  );
}
