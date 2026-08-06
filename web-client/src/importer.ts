import { createTransaction, type JsonEntity } from "./core";

export interface ImportPreview {
  rows: JsonEntity[];
  warnings: string[];
  sourceType: string;
}

function normalizeHeader(value: unknown) {
  return String(value ?? "").trim().toLocaleLowerCase().replace(/\s+/g, "");
}

function pick(row: Record<string, unknown>, names: string[]) {
  for (const name of names) {
    const key = Object.keys(row).find((candidate) => normalizeHeader(candidate) === normalizeHeader(name));
    if (key && row[key] !== undefined && row[key] !== null && String(row[key]).trim()) return row[key];
  }
  return null;
}

export function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let value = "";
  let quoted = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index]!;
    if (character === '"') {
      if (quoted && text[index + 1] === '"') { value += '"'; index += 1; }
      else quoted = !quoted;
    } else if (character === "," && !quoted) {
      row.push(value); value = "";
    } else if ((character === "\n" || character === "\r") && !quoted) {
      if (character === "\r" && text[index + 1] === "\n") index += 1;
      row.push(value); value = "";
      if (row.some((item) => item.trim())) rows.push(row);
      row = [];
    } else value += character;
  }
  row.push(value);
  if (row.some((item) => item.trim())) rows.push(row);
  return rows;
}

function rowsToObjects(rows: unknown[][]) {
  const [headers = [], ...body] = rows;
  return body.map((values) => Object.fromEntries(headers.map((header, index) => [String(header), values[index]])));
}

function inferSource(fileName: string, headers: string[]) {
  const combined = `${fileName} ${headers.join(" ")}`.toLocaleLowerCase();
  if (combined.includes("微信") || combined.includes("wechat")) return "wechat_import";
  if (combined.includes("支付宝") || combined.includes("alipay")) return "alipay_import";
  if (combined.includes("银行") || combined.includes("bank")) return "bank_import";
  return "file_import";
}

function normalizeAmount(value: unknown) {
  const cleaned = String(value ?? "").replace(/[￥¥,\s]/g, "").replace(/^\+/, "");
  if (!cleaned) throw new Error("缺少金额");
  return cleaned;
}

function normalizeDate(value: unknown) {
  if (value instanceof Date) return value.toISOString();
  const raw = String(value ?? "").trim().replace(/\//g, "-");
  const parsed = new Date(raw);
  if (Number.isNaN(parsed.getTime())) throw new Error("日期格式无法识别");
  return parsed.toISOString();
}

export function mapImportRows(
  userId: string,
  deviceId: string,
  objects: Array<Record<string, unknown>>,
  sourceType: string,
  accountId?: string | null,
): ImportPreview {
  const rows: JsonEntity[] = [];
  const warnings: string[] = [];
  objects.forEach((row, index) => {
    try {
      const direction = String(pick(row, ["收/支", "收支类型", "交易类型", "type"]) ?? "支出");
      const status = String(pick(row, ["当前状态", "交易状态", "status"]) ?? "");
      const amount = normalizeAmount(pick(row, ["金额(元)", "金额", "交易金额", "amount"]));
      const occurredAt = normalizeDate(pick(row, ["交易时间", "时间", "日期", "occurredat"]));
      const type = /收入|收款|income/i.test(direction) ? "income" : /退款|refund/i.test(direction) ? "refund" : "expense";
      rows.push(createTransaction(userId, deviceId, {
        accountId: accountId ?? null,
        amount,
        type,
        occurredAt,
        localDate: occurredAt.slice(0, 10),
        status: /成功|完成|confirmed/i.test(status) ? "confirmed" : "candidate",
        sourceType,
        merchant: String(pick(row, ["交易对方", "商户名称", "交易对象", "merchant"]) ?? "") || null,
        item: String(pick(row, ["商品", "商品说明", "交易内容", "item"]) ?? "") || null,
        note: String(pick(row, ["备注", "note"]) ?? "") || null,
        externalTransactionId: String(pick(row, ["交易单号", "订单号", "流水号", "transactionid"]) ?? "") || null,
      }));
    } catch (error) {
      warnings.push(`第 ${index + 2} 行：${error instanceof Error ? error.message : "无法解析"}`);
    }
  });
  return { rows, warnings, sourceType };
}

export async function importBillFile(userId: string, deviceId: string, file: File, accountId?: string | null): Promise<ImportPreview> {
  let rows: unknown[][];
  if (/\.xlsx?$/i.test(file.name)) {
    const { default: readXlsxFile } = await import("read-excel-file/browser");
    rows = await readXlsxFile(file);
  } else {
    const text = await file.text();
    rows = parseCsv(text);
  }
  const objects = rowsToObjects(rows);
  const sourceType = inferSource(file.name, Object.keys(objects[0] ?? {}));
  return mapImportRows(userId, deviceId, objects, sourceType, accountId);
}
