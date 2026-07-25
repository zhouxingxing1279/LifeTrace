import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

test("defines finished HengXu metadata instead of the starter preview", async () => {
  const layout = await readFile(new URL("../app/layout.tsx", import.meta.url), "utf8");
  assert.match(layout, /<html lang="zh-CN">/);
  assert.match(layout, /Life trace — 个人管理平台/);
  assert.match(layout, /坚持、训练、财务与复盘/);
  assert.doesNotMatch(layout, /manifest:\s*"\/manifest\.webmanifest"/);
  assert.doesNotMatch(layout, /maximumScale:\s*1/);
  assert.doesNotMatch(layout, /userScalable:\s*false/);
  assert.doesNotMatch(layout, /codex-preview|Your site is taking shape|Building your site/i);
});

test("ships the completed application shell and local-first capabilities", async () => {
  const [page, layout, appShell, store, database, apiRoute, hosting] = await Promise.all([
    readFile(new URL("../app/page.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/layout.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/components/HengXuShell.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/stores/useLifeStore.ts", import.meta.url), "utf8"),
    readFile(new URL("../db/schema.ts", import.meta.url), "utf8"),
    readFile(new URL("../app/api/state/route.ts", import.meta.url), "utf8"),
    readFile(new URL("../.openai/hosting.json", import.meta.url), "utf8"),
  ]);

  assert.match(page, /<HengXuShell\s*\/>/);
  assert.match(layout, /title:\s*["']Life trace/);
  assert.match(database, /sqliteTable/);
  assert.match(database, /activity_logs/);
  assert.match(apiRoute, /CREATE TABLE IF NOT EXISTS/);
  assert.match(apiRoute, /ON CONFLICT\(id\) DO UPDATE/);
  assert.match(hosting, /"d1":\s*"DB"/);
  assert.match(store, /restoreBackup/);
  assert.match(store, /mutateSQLite/);
  assert.doesNotMatch(store, /Dexie|IndexedDB|db\.activities/);
  assert.doesNotMatch(appShell, /训练模板|新建模板|TemplateForm/);
  assert.doesNotMatch(store, /workoutTemplates|WorkoutTemplate/);
  assert.doesNotMatch(apiRoute, /workout_templates|workoutTemplates/);

  for (const feature of [
    "坚持项目",
    "健身训练",
    "账单管理",
    "账户管理",
    "账单导入",
    "每日复盘",
    "SQLite",
    "导出备份",
  ]) {
    assert.match(appShell, new RegExp(feature));
  }

  await assert.rejects(access(new URL("../app/_sites-preview", import.meta.url)));
});

test("provides an installable Chinese web-app manifest", async () => {
  const [manifest, fitnessManifest, rootLayout, fitnessLayout, fitnessPage, fitnessApp] = await Promise.all([
    readFile(new URL("../app/manifest.ts", import.meta.url), "utf8"),
    readFile(new URL("../app/fitness/manifest.ts", import.meta.url), "utf8"),
    readFile(new URL("../app/layout.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/fitness/layout.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/fitness/page.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/components/FitnessPwaApp.tsx", import.meta.url), "utf8"),
  ]);
  assert.match(manifest, /name:\s*"Life trace 导入"/);
  assert.match(manifest, /display:\s*"standalone"/);
  assert.match(manifest, /start_url:\s*"\/fitness"/);
  assert.match(manifest, /scope:\s*"\/fitness"/);
  assert.match(manifest, /src:\s*"\/favicon\.svg"/);
  assert.match(fitnessManifest, /name:\s*"Life trace 导入"/);
  assert.match(fitnessManifest, /id:\s*"\/fitness"/);
  assert.match(fitnessManifest, /start_url:\s*"\/fitness"/);
  assert.match(fitnessManifest, /scope:\s*"\/fitness"/);
  assert.doesNotMatch(rootLayout, /manifest:\s*"\/manifest\.webmanifest"/);
  assert.match(fitnessLayout, /manifest:\s*"\/manifest\.webmanifest"/);
  assert.match(fitnessPage, /redirect\("\/"\)/);
  assert.match(fitnessPage, /<FitnessPwaApp\s*\/>/);
  for (const feature of ["手机数据入口", "导入训练数据", "导入账单数据", "电脑端解析", "发送记录"]) {
    assert.match(fitnessApp, new RegExp(feature));
  }
});

test("moves phone uploads to computer-backed D1 and R2 processing", async () => {
  const [route, desktop, hosting] = await Promise.all([
    readFile(new URL("../app/api/imports/route.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/components/HengXuShell.tsx", import.meta.url), "utf8"),
    readFile(new URL("../.openai/hosting.json", import.meta.url), "utf8"),
  ]);
  assert.match(hosting, /"r2":\s*"UPLOADS"/);
  assert.match(route, /env\.UPLOADS\.put/);
  assert.match(route, /import_uploads/);
  assert.match(desktop, /待处理导入文件/);
  assert.match(desktop, /电脑解析/);
});

test("keeps the phone app upload-only and removes its cache service", async () => {
  const [serviceWorker, fitnessApp, manager, healthRoute] = await Promise.all([
    readFile(new URL("../public/sw.js", import.meta.url), "utf8"),
    readFile(new URL("../src/components/FitnessPwaApp.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/components/PwaManager.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/api/health/route.ts", import.meta.url), "utf8"),
  ]);

  assert.match(fitnessApp, /\/api\/xunji\/parse/);
  assert.match(fitnessApp, /\/api\/imports/);
  assert.doesNotMatch(fitnessApp, /useLifeStore|deviceFitnessStorage|indexedDB|localStorage|训练模板|动作库|训练历史/);
  assert.doesNotMatch(manager, /serviceWorker\.register|PREPARE_OFFLINE|navigator\.storage\.persist/);
  assert.match(manager, /registration\.unregister/);
  assert.match(manager, /fetch\("\/api\/health"/);
  assert.match(manager, /fitnessMode && connection === "disconnected"/);
  assert.doesNotMatch(manager, /navigator\.onLine/);
  assert.match(healthRoute, /service:\s*"lifetrace-upload"/);
  assert.match(healthRoute, /cache-control.*no-store/);
  assert.match(serviceWorker, /registration\.unregister/);
  assert.doesNotMatch(serviceWorker, /addEventListener\("fetch"|cache\.put|cache\.match/);
});

test("provides the complete persistent daily English learning loop", async () => {
  const [shell, page, schema, repository, analysisService, migration, source, syncRoute, englishTypes] = await Promise.all([
    readFile(new URL("../src/components/HengXuShell.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/components/english/DailyEnglish.tsx", import.meta.url), "utf8"),
    readFile(new URL("../db/schema.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/server/englishRepository.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/services/englishAnalysis.ts", import.meta.url), "utf8"),
    readFile(new URL("../drizzle/0004_calm_kulan_gath.sql", import.meta.url), "utf8"),
    readFile(new URL("../src/server/englishSources/voa.ts", import.meta.url), "utf8"),
    readFile(new URL("../app/api/english/sync/route.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/types/english.ts", import.meta.url), "utf8"),
  ]);

  assert.match(shell, /id:\s*"english",\s*label:\s*"每日英语"/);
  assert.match(shell, /<DailyEnglish\s*\/>/);
  for (const feature of ["今日任务", "文章库", "生词本", "学习记录", "AI 助手", "开始英文总结", "参考版本"]) {
    assert.match(page, new RegExp(feature));
  }
  for (const table of ["english_articles", "english_learning_records", "english_vocabulary", "english_highlights", "english_notes", "english_ai_analysis"]) {
    assert.match(schema, new RegExp(table));
    assert.match(migration, new RegExp(table));
  }
  assert.match(analysisService, /interface EnglishAnalysisService/);
  assert.match(analysisService, /class MockEnglishAnalysisService/);
  assert.match(repository, /ensureEnglishHabitLog/);
  assert.match(repository, /nextReviewTime/);
  assert.match(repository, /syncVoaArticles/);
  assert.match(page, /同步 VOA/);
  assert.match(page, /VOA Learning English/);
  assert.match(source, /learningenglish\.voanews\.com\/api\//);
  assert.match(source, /isVoaOwnedArticle/);
  assert.match(source, /Associated Press\|Agence France-Presse\|Reuters\|AFP/);
  assert.match(source, /safeVoaUrl/);
  assert.match(syncRoute, /syncVoaArticles/);
  for (const field of ["sourceUrl", "externalId", "publishedAt", "sourceName", "audioUrl", "author", "summary", "wordCount", "fetchedAt", "rightsNote"]) {
    assert.match(englishTypes, new RegExp(`${field}\\?`));
  }
  assert.match(source, /parseArticleJsonLd/);
  assert.match(source, /FETCH_ATTEMPTS\s*=\s*3/);
  assert.match(source, /zmypyl-vomx-tpeyry_/);
  assert.match(source, /audioFromHtml/);
  assert.match(page, /VOA 原文音频/);
  assert.match(page, /preload="none"/);
});

test("adds a confirmation-first Xunji workout import pipeline without OCR", async () => {
  const [panel, repository, proxy, pythonApi, decoder, parser, migration] = await Promise.all([
    readFile(new URL("../src/components/XunjiImportPanel.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/server/xunjiImportRepository.ts", import.meta.url), "utf8"),
    readFile(new URL("../app/api/xunji/parse/route.ts", import.meta.url), "utf8"),
    readFile(new URL("../xunji_service/app/main.py", import.meta.url), "utf8"),
    readFile(new URL("../xunji_service/app/qr_decoder.py", import.meta.url), "utf8"),
    readFile(new URL("../xunji_service/app/parser.py", import.meta.url), "utf8"),
    readFile(new URL("../drizzle/0005_fearless_sister_grimm.sql", import.meta.url), "utf8"),
  ]);

  for (const text of ["训记训练数据同步", "解析成功", "确认导入", "编辑训练数据", "图片仅用于读取二维码"]) {
    assert.match(panel, new RegExp(text));
  }
  assert.match(proxy, /XUNJI_SERVICE_URL/);
  assert.match(proxy, /api\.xunjiapp\.cn/);
  assert.match(pythonApi, /parse_embedded_json/);
  assert.match(pythonApi, /parse_with_playwright/);
  assert.match(pythonApi, /parse_dom/);
  assert.match(decoder, /QRCodeDetector/);
  assert.match(decoder, /pyzbar/);
  assert.doesNotMatch(decoder, /OCR|tesseract|easyocr/i);
  assert.match(parser, /page\.html/);
  assert.match(parser, /network\.json/);
  assert.match(parser, /response\.json/);
  assert.match(repository, /status:\s*"pending"/);
  assert.match(repository, /source:\s*"xunji"/);
  assert.match(repository, /activity_logs/);
  assert.match(repository, /training_notes/);
  assert.match(migration, /workout_import_records/);
  assert.match(migration, /training_notes/);
  assert.doesNotMatch(
    await readFile(new URL("../src/components/HengXuShell.tsx", import.meta.url), "utf8"),
    /id:\s*"exercises",\s*label:\s*"动作资料库"/,
  );
});

test("ships a persistent, secure Electron notes workspace", async () => {
  const [shell, module, route, validation, preload, main, migration, styles] = await Promise.all([
    readFile(new URL("../src/components/HengXuShell.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/components/NotesModule.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/api/notes/route.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/server/noteSchemas.ts", import.meta.url), "utf8"),
    readFile(new URL("../desktop/preload.cjs", import.meta.url), "utf8"),
    readFile(new URL("../desktop/main.cjs", import.meta.url), "utf8"),
    readFile(new URL("../drizzle/0007_notes_module.sql", import.meta.url), "utf8"),
    readFile(new URL("../app/notes.css", import.meta.url), "utf8"),
  ]);

  assert.match(shell, /id:\s*"notes",\s*label:\s*"笔记"/);
  assert.match(shell, /<NotesModule\s*\/>/);
  assert.match(shell, /<DashboardNotes/);
  for (const feature of ["@tiptap/react", "DOMPurify", "setTimeout\\(\\(\\)=>void save\\(false\\),800\\)", "快速记录", "回收站", "版本历史", "关联数据", "附件"]) {
    assert.match(module, new RegExp(feature));
  }
  for (const table of ["notes", "note_folders", "note_tags", "note_tag_relations", "note_relations", "note_attachments", "note_revisions", "notes_fts"]) {
    assert.match(migration, new RegExp(table));
    assert.match(route, new RegExp(table));
  }
  assert.match(route, /LIKE \? ESCAPE/);
  assert.match(route, /LIMIT \?/);
  assert.match(route, /notePayloadSchema\.parse/);
  assert.match(validation, /z\.enum/);
  assert.match(validation, /idSchema/);
  assert.match(preload, /contextBridge\.exposeInMainWorld\("noteApi"/);
  assert.doesNotMatch(preload, /require\("node:fs"\)|require\("node:path"\)/);
  assert.match(main, /contextIsolation:\s*true/);
  assert.match(main, /nodeIntegration:\s*false/);
  assert.match(main, /attachmentLimit\s*=\s*20\s*\*\s*1024\s*\*\s*1024/);
  assert.match(main, /path\.dirname\(target\)\s*!==\s*path\.resolve\(directory\)/);
  assert.match(main, /allowedNoteFiles/);
  assert.match(styles, /grid-template-columns/);
  assert.match(styles, /cursor:col-resize/);
});

test("shares one compatible persist-project editor with live personalization", async () => {
  const [shell, dialog, controls, model, types, store, workoutSync, englishSync, styles] = await Promise.all([
    readFile(new URL("../src/components/HengXuShell.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/components/persist-project/PersistProjectDialog.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/components/persist-project/ProjectControls.tsx", import.meta.url), "utf8"),
    readFile(new URL("../src/components/persist-project/projectModel.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/types/index.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/stores/useLifeStore.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/server/xunjiImportRepository.ts", import.meta.url), "utf8"),
    readFile(new URL("../src/server/englishRepository.ts", import.meta.url), "utf8"),
    readFile(new URL("../app/persist-project.css", import.meta.url), "utf8"),
  ]);

  assert.match(shell, /<PersistProjectDialog/);
  assert.doesNotMatch(shell, /function ActivityForm/);
  assert.match(dialog, /mode = activity \? "edit" : "create"/);
  assert.match(dialog, /ProjectLivePreview/);
  assert.match(dialog, /window\.confirm/);
  assert.match(dialog, /focusableSelector/);
  assert.match(controls, /更多图标/);
  assert.match(controls, /PROJECT_COLORS/);
  assert.match(model, /projectDraftToActivity/);
  assert.match(model, /validateProjectDraft/);
  for (const field of ["color", "scheduleType", "startDate", "checkinMethod", "syncSource"]) {
    assert.match(types, new RegExp(`${field}\\?`));
    assert.match(store, new RegExp(field));
  }
  assert.match(workoutSync, /syncSource === "fitness"/);
  assert.match(englishSync, /syncSource === "english"/);
  assert.match(styles, /grid-template-columns:\s*minmax\(330px,\s*35%\)\s*minmax\(0,\s*1fr\)/);
  assert.match(styles, /prefers-reduced-motion/);
});
