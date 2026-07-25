const { app, BrowserWindow, dialog, shell, ipcMain, Menu } = require("electron");
const { spawn, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const net = require("node:net");
const path = require("node:path");
const { z } = require("zod");

const projectRoot = path.resolve(__dirname, "..");
const appUrl = "http://127.0.0.1:3103/";
const loadingPage = path.join(__dirname, "loading.html");
const logPath = path.join(projectRoot, ".desktop-runtime.log");
const ownedProcesses = new Map();
const ownedPorts = new Set();
const schedulerTimers = new Set();

let mainWindow = null;
let isQuitting = false;
const noteIdSchema = z.string().min(1).max(100).regex(/^[\w-]+$/);
const storedFileSchema = z.string().min(1).max(260).regex(/^[^/\\]+$/);
const allowedNoteFiles = new Set([".jpg", ".jpeg", ".png", ".webp", ".gif", ".pdf", ".txt", ".md", ".docx", ".xlsx"]);
const attachmentLimit = 20 * 1024 * 1024;

function noteDirectory(noteId) {
  return path.join(app.getPath("userData"), "attachments", "notes", noteIdSchema.parse(noteId));
}

function cleanFileName(value) {
  const extension = path.extname(value).toLowerCase();
  const base = path.basename(value, extension).replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_").slice(0, 120) || "attachment";
  return `${base}${extension}`;
}

function uniqueFilePath(directory, originalName) {
  const safe = cleanFileName(originalName);
  const extension = path.extname(safe);
  const base = path.basename(safe, extension);
  let candidate = path.join(directory, safe);
  let index = 2;
  while (fs.existsSync(candidate)) candidate = path.join(directory, `${base}-${index++}${extension}`);
  return candidate;
}

function registerNoteIpc() {
  ipcMain.handle("notes:select-attachment", async (_event, payload) => {
    try {
      const noteId = noteIdSchema.parse(payload?.noteId);
      const result = await dialog.showOpenDialog(mainWindow, {
        title: "添加笔记附件", properties: ["openFile"],
        filters: [{ name: "支持的文件", extensions: [...allowedNoteFiles].map((item) => item.slice(1)) }],
      });
      if (result.canceled || !result.filePaths[0]) return { ok: true, canceled: true };
      const source = path.resolve(result.filePaths[0]);
      const extension = path.extname(source).toLowerCase();
      if (!allowedNoteFiles.has(extension)) throw new Error("不支持此文件类型");
      const stat = await fs.promises.stat(source);
      if (!stat.isFile() || stat.size > attachmentLimit) throw new Error("附件必须是小于 20 MB 的普通文件");
      const directory = noteDirectory(noteId);
      await fs.promises.mkdir(directory, { recursive: true });
      const destination = uniqueFilePath(directory, path.basename(source));
      await fs.promises.copyFile(source, destination);
      return { ok: true, file: { id: crypto.randomUUID(), noteId, fileName: path.basename(destination), originalName: path.basename(source), mimeType: extension.slice(1), fileSize: stat.size, storagePath: destination } };
    } catch (error) { return { ok: false, error: error.message }; }
  });
  const resolveStored = (payload) => {
    const directory = noteDirectory(payload?.noteId);
    const fileName = storedFileSchema.parse(payload?.fileName);
    const target = path.resolve(directory, fileName);
    if (path.dirname(target) !== path.resolve(directory)) throw new Error("附件路径无效");
    return target;
  };
  ipcMain.handle("notes:open-attachment", async (_event, payload) => {
    try { const target = resolveStored(payload); if (!fs.existsSync(target)) throw new Error("附件文件已不存在"); const error = await shell.openPath(target); if (error) throw new Error(error); return { ok:true }; }
    catch (error) { return { ok:false,error:error.message }; }
  });
  ipcMain.handle("notes:show-attachment", async (_event, payload) => {
    try { const target=resolveStored(payload); if(!fs.existsSync(target))throw new Error("附件文件已不存在"); shell.showItemInFolder(target); return {ok:true}; }
    catch(error){ return {ok:false,error:error.message}; }
  });
  ipcMain.handle("notes:delete-attachment", async (_event, payload) => {
    try { const target=resolveStored(payload); if(fs.existsSync(target))await fs.promises.unlink(target); return {ok:true}; }
    catch(error){ return {ok:false,error:error.message}; }
  });
  ipcMain.handle("notes:export", async (_event, payload) => {
    try {
      const format = z.enum(["md","html","json"]).parse(payload?.format);
      const title = cleanFileName(String(payload?.title || "无标题笔记")).replace(/\.[^.]+$/, "");
      const content = z.string().max(10_000_000).parse(payload?.content);
      const result = await dialog.showSaveDialog(mainWindow, { title:"导出笔记", defaultPath:`${title}.${format}`, filters:[{name:format.toUpperCase(),extensions:[format]}] });
      if(result.canceled||!result.filePath)return {ok:true,canceled:true};
      await fs.promises.writeFile(result.filePath, content, "utf8");
      return {ok:true,filePath:result.filePath};
    } catch(error){ return {ok:false,error:error.message}; }
  });
  ipcMain.handle("notes:import-markdown", async () => {
    try {
      const result=await dialog.showOpenDialog(mainWindow,{title:"导入 Markdown",properties:["openFile"],filters:[{name:"Markdown",extensions:["md","markdown"]}]});
      if(result.canceled||!result.filePaths[0])return {ok:true,canceled:true};
      const stat=await fs.promises.stat(result.filePaths[0]); if(stat.size>5*1024*1024)throw new Error("Markdown 文件不能超过 5 MB");
      return {ok:true,title:path.basename(result.filePaths[0],path.extname(result.filePaths[0])),content:await fs.promises.readFile(result.filePaths[0],"utf8")};
    } catch(error){return {ok:false,error:error.message};}
  });
}

function installApplicationMenu() {
  const send = (command) => mainWindow?.webContents.send("notes:command", command);
  const template = [
    { label:"笔记",submenu:[
      {label:"新建笔记",accelerator:"CmdOrCtrl+N",click:()=>send("new")},
      {label:"新建快速记录",accelerator:"CmdOrCtrl+Shift+N",click:()=>send("quick")},
      {label:"导入 Markdown",click:()=>send("import")},
      {type:"separator"},{label:"保存",accelerator:"CmdOrCtrl+S",click:()=>send("save")},
      {label:"搜索笔记",accelerator:"CmdOrCtrl+Shift+F",click:()=>send("search")},
      {label:"打开笔记模块",accelerator:"CmdOrCtrl+Alt+1",click:()=>send("open")},
      {type:"separator"},{label:"收藏当前笔记",click:()=>send("favorite")},
      {label:"置顶当前笔记",click:()=>send("pin")},
      {label:"导出当前笔记",click:()=>send("export")},
      {label:"删除当前笔记",click:()=>send("trash")},
    ]},
    {role:"editMenu"},{role:"viewMenu"},{role:"windowMenu"},
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

function writeLog(message) {
  const line = `[${new Date().toISOString()}] ${message}\n`;
  try {
    fs.appendFileSync(logPath, line, "utf8");
  } catch {
    // Logging must never prevent the desktop app from opening.
  }
}

function isPortOpen(port, host = "127.0.0.1", timeout = 700) {
  return new Promise((resolve) => {
    const socket = net.createConnection({ port, host });
    const finish = (result) => {
      socket.removeAllListeners();
      socket.destroy();
      resolve(result);
    };
    socket.setTimeout(timeout);
    socket.once("connect", () => finish(true));
    socket.once("timeout", () => finish(false));
    socket.once("error", () => finish(false));
  });
}

function requestOk(url, timeout = 1500) {
  return new Promise((resolve) => {
    const request = http.get(url, (response) => {
      response.resume();
      resolve(Boolean(response.statusCode && response.statusCode < 500));
    });
    request.setTimeout(timeout, () => {
      request.destroy();
      resolve(false);
    });
    request.once("error", () => resolve(false));
  });
}

function postJson(url, payload, timeout = 5000) {
  return new Promise((resolve) => {
    const target = new URL(url);
    const body = Buffer.from(JSON.stringify(payload));
    const request = http.request({
      hostname: target.hostname,
      port: target.port,
      path: target.pathname + target.search,
      method: "POST",
      headers: { "content-type": "application/json", "content-length": body.length },
    }, (response) => {
      response.resume();
      resolve(Boolean(response.statusCode && response.statusCode < 500));
    });
    request.setTimeout(timeout, () => { request.destroy(); resolve(false); });
    request.once("error", () => resolve(false));
    request.end(body);
  });
}

function installEnglishScheduler() {
  const trigger = async (pathName, payload, label) => {
    const ok = await postJson(new URL(pathName, appUrl).toString(), payload);
    writeLog(`英语文章${label}${ok ? "已触发" : "触发失败，将在下一周期重试"}`);
  };
  void trigger("/api/english/sync", { startupCheck: true }, "启动检查");
  schedulerTimers.add(setInterval(() => {
    void trigger("/api/english/sync", { startupCheck: true }, "增量同步");
  }, 24 * 60 * 60 * 1000));
  schedulerTimers.add(setInterval(() => {
    void trigger("/api/english/sync/repair", { deep: false }, "周度补漏");
  }, 7 * 24 * 60 * 60 * 1000));
  schedulerTimers.add(setInterval(() => {
    void trigger("/api/english/sync/repair", { deep: true }, "月度健康检查");
  }, 30 * 24 * 60 * 60 * 1000));
}

async function waitFor(check, timeoutMs, intervalMs = 500) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await check()) return true;
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  return false;
}

function startProcess(name, executable, args, extra = {}) {
  writeLog(`启动 ${name}: ${executable} ${args.join(" ")}`);
  const child = spawn(executable, args, {
    cwd: projectRoot,
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
    ...extra,
  });

  ownedProcesses.set(name, child);
  child.stdout?.on("data", (chunk) => writeLog(`${name}: ${String(chunk).trimEnd()}`));
  child.stderr?.on("data", (chunk) => writeLog(`${name}: ${String(chunk).trimEnd()}`));
  child.once("error", (error) => writeLog(`${name} 启动失败: ${error.message}`));
  child.once("exit", (code, signal) => {
    writeLog(`${name} 已退出，代码=${code ?? "无"}，信号=${signal ?? "无"}`);
    if (ownedProcesses.get(name) === child) ownedProcesses.delete(name);
  });
  return child;
}

function stopOwnedProcesses() {
  for (const [name, child] of ownedProcesses) {
    if (!child.pid || child.killed) continue;
    writeLog(`停止 ${name}，PID=${child.pid}`);
    if (process.platform === "win32") {
      spawnSync("taskkill.exe", ["/pid", String(child.pid), "/T", "/F"], {
        windowsHide: true,
        stdio: "ignore",
      });
    } else {
      child.kill("SIGTERM");
    }
  }
  ownedProcesses.clear();

  if (process.platform === "win32") {
    for (const port of ownedPorts) {
      const result = spawnSync("powershell.exe", [
        "-NoProfile",
        "-Command",
        `(Get-NetTCPConnection -State Listen -LocalPort ${port} -ErrorAction SilentlyContinue).OwningProcess`,
      ], {
        windowsHide: true,
        encoding: "utf8",
      });
      const pids = String(result.stdout || "")
        .split(/\s+/)
        .map(Number)
        .filter((pid) => Number.isInteger(pid) && pid > 0 && pid !== process.pid);
      for (const pid of new Set(pids)) {
        writeLog(`停止端口 ${port} 的后台进程，PID=${pid}`);
        spawnSync("taskkill.exe", ["/pid", String(pid), "/T", "/F"], {
          windowsHide: true,
          stdio: "ignore",
        });
      }
    }
  }
  ownedPorts.clear();
}

async function ensureBuild() {
  const wranglerConfig = path.join(projectRoot, "dist", "server", "wrangler.json");
  if (fs.existsSync(wranglerConfig)) return;

  const npmExecutable = process.platform === "win32" ? "npm.cmd" : "npm";
  const build = startProcess("首次构建", npmExecutable, ["run", "pwa:build"]);
  const completed = await new Promise((resolve) => {
    build.once("exit", (code) => resolve(code === 0));
    build.once("error", () => resolve(false));
  });
  if (!completed || !fs.existsSync(wranglerConfig)) {
    throw new Error("首次构建没有完成，请查看 .desktop-runtime.log。");
  }
}

async function ensureMainService() {
  if (await requestOk(appUrl)) {
    writeLog("检测到主服务已经运行，直接连接。");
    return;
  }

  await ensureBuild();
  const nodeExecutable = process.env.LIFETRACE_NODE || "node";
  const wranglerCli = path.join(projectRoot, "node_modules", "wrangler", "bin", "wrangler.js");
  startProcess("Life trace 主服务", nodeExecutable, [
    wranglerCli,
    "dev",
    "--config",
    path.join(projectRoot, "dist", "server", "wrangler.json"),
    "--port",
    "3103",
    "--ip",
    "0.0.0.0",
    "--persist-to",
    path.join(projectRoot, ".wrangler", "state"),
  ]);
  ownedPorts.add(3103);

  const ready = await waitFor(() => requestOk(appUrl), 90000);
  if (!ready) throw new Error("主服务启动超时，请查看 .desktop-runtime.log。");
}

async function ensureOptionalServices() {
  if (!(await isPortOpen(8001))) {
    const venvPython = path.join(projectRoot, ".venv-xunji", "Scripts", "python.exe");
    const pythonExecutable = fs.existsSync(venvPython)
      ? venvPython
      : (process.env.LIFETRACE_PYTHON || "python");
    startProcess("训记解析服务", pythonExecutable, [
      path.join(projectRoot, "xunji_service", "run.py"),
    ]);
    ownedPorts.add(8001);
  } else {
    writeLog("检测到训记解析服务已经运行。");
  }

  const certificate = path.join(projectRoot, ".local-certs", "lifetrace-local.pfx");
  if (fs.existsSync(certificate) && !(await isPortOpen(3443))) {
    const nodeExecutable = process.env.LIFETRACE_NODE || "node";
    startProcess("手机同步服务", nodeExecutable, [
      path.join(projectRoot, "scripts", "local-https.mjs"),
    ]);
    ownedPorts.add(3443);
  } else if (await isPortOpen(3443)) {
    writeLog("检测到手机同步服务已经运行。");
  }
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1460,
    height: 920,
    minWidth: 1050,
    minHeight: 680,
    title: "Life trace",
    icon: path.join(projectRoot, "public", "icons", "icon-512.png"),
    backgroundColor: "#f4f5f1",
    autoHideMenuBar: true,
    show: false,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: path.join(__dirname, "preload.cjs"),
    },
  });

  mainWindow.setMenuBarVisibility(false);
  mainWindow.loadFile(loadingPage);
  mainWindow.once("ready-to-show", () => mainWindow?.show());

  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith(appUrl)) return { action: "allow" };
    shell.openExternal(url);
    return { action: "deny" };
  });
  mainWindow.webContents.on("will-navigate", (event, url) => {
    if (!url.startsWith(appUrl)) {
      event.preventDefault();
      shell.openExternal(url);
    }
  });
  mainWindow.webContents.on("before-input-event", (event, input) => {
    const isZoomShortcut = (input.control || input.meta)
      && ["+", "-", "=", "0"].includes(input.key);
    if (isZoomShortcut) event.preventDefault();
  });
  mainWindow.on("closed", () => {
    mainWindow = null;
  });
}

async function startApplication() {
  try {
    await ensureMainService();
    await ensureOptionalServices();
    await mainWindow?.loadURL(appUrl);
    mainWindow?.webContents.setVisualZoomLevelLimits(1, 1);
    installEnglishScheduler();
    writeLog("Life trace 桌面 App 已就绪。");
  } catch (error) {
    writeLog(`桌面 App 启动失败: ${error.stack || error.message}`);
    await dialog.showMessageBox({
      type: "error",
      title: "Life trace 启动失败",
      message: "Life trace 暂时无法启动",
      detail: `${error.message}\n\n错误记录：${logPath}`,
      buttons: ["关闭"],
    });
    app.quit();
  }
}

const gotSingleInstanceLock = app.requestSingleInstanceLock();
if (!gotSingleInstanceLock) {
  app.quit();
} else {
  app.on("second-instance", () => {
    if (!mainWindow) return;
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.show();
    mainWindow.focus();
  });

  app.whenReady().then(() => {
    registerNoteIpc();
    installApplicationMenu();
    createWindow();
    void startApplication();
  });
}

app.on("before-quit", () => {
  if (isQuitting) return;
  isQuitting = true;
  for (const timer of schedulerTimers) clearInterval(timer);
  schedulerTimers.clear();
  stopOwnedProcesses();
});

app.on("window-all-closed", () => {
  app.quit();
});
