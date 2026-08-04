# EPIC-01 目标 Schema（规范化表）

> 版本：0.2.1 + EPIC-01（Migration 版本 1-6）
> 约定：时间点统一 UTC RFC3339；业务自然日 `YYYY-MM-DD`；金额整数分；`user_id` 默认 `local`；
> 主实体统一含 `created_at / updated_at / deleted_at / version / modified_by_device`。

## 1. 财务

### finance_accounts

| 列 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 旧 ID 原样保留 |
| user_id | TEXT | 默认 local |
| name | TEXT NOT NULL | 账户名 |
| account_type | TEXT CHECK | cash/bank/wechat/alipay/investment/other |
| opening_balance_cents | INTEGER | 旧 balance × 100 |
| balance_at / last4 / color / icon | TEXT | 展示字段 |
| is_archived | INTEGER | 0/1 |
| created_at / updated_at / deleted_at / version / modified_by_device | | 公共字段 |

### transaction_categories

`id, user_id, name, category_type(expense/income/transfer), parent_id, icon, color, is_system, is_archived, 公共字段`
唯一索引：`(user_id, category_type, name) WHERE deleted_at IS NULL`。

### transactions

`id, user_id, transaction_type(expense/income/transfer/refund/fee), amount_cents INTEGER NOT NULL CHECK>=0,
currency, account_id, to_account_id, category_id, counterparty, merchant, item, note, occurred_at,
local_date, status(candidate/provisional/confirmed/ignored), source_type, external_transaction_id,
legacy_category_name, legacy_account_name, raw_json, 公共字段`

外键：account_id/to_account_id → finance_accounts；category_id → transaction_categories。
索引：date/account/category；唯一：`(user_id, source_type, external_transaction_id)`（非空且未删除）。

### transaction_evidence

`id, transaction_id(FK CASCADE), source_type, source_id, external_transaction_id, confidence, raw_json, created_at`

## 2. 习惯与复盘

### activities

`id, user_id, name, activity_type(duration/count/completion/weekly/control), unit, minimum_target,
normal_target, target_period(daily/weekly), target_days_json, icon, color, schedule_type,
start_date, checkin_method, sync_source, description, is_archived, 公共字段`

### activity_logs

`id, user_id, activity_id(FK→activities, 可空), log_date, value, status, note, metadata_json, 公共字段`
索引：`(activity_id, log_date, deleted_at)`、`(log_date, deleted_at)`。

### daily_reviews

`id, user_id, review_date, energy/mood(1-10), completion_score, best_thing, problem,
tomorrow_priority, note, 公共字段`
唯一索引：`(user_id, review_date) WHERE deleted_at IS NULL`（同日多条旧复盘软删除保留 + issue）。

## 3. 笔记

### note_folders / note_tags

`id, user_id, name, icon/color, sort_order(文件夹), 公共字段`；标签唯一 `(user_id, name) WHERE deleted_at IS NULL`。

### notes

`id, user_id, title, note_type, folder_id(FK), content_json, content_html, content_text,
content_markdown, summary, is_pinned, is_favorite, is_archived, ai_summary, ai_tags_json,
embedding_status, last_ai_processed_at, 公共字段`

列表查询不读取 `content_json/content_html/content_markdown`；详情才读取正文。

### note_tag_relations / note_relations / note_attachments / note_revisions

- `note_tag_relations(note_id, tag_id, created_at)` 复合主键，双外键 CASCADE
- `note_relations(id, note_id FK, entity_type, entity_id, relation_type, created_at)`
- `note_attachments(id, note_id FK, file_name, original_name, mime_type, file_size, storage_path, created_at)`
- `note_revisions(id, note_id FK, revision_version, title, content_json, content_html, content_markdown, created_at)`，唯一 `(note_id, revision_version)`

FTS5：`notes_fts(title, content_text, summary, note_id UNINDEXED)`，不可用时回退参数化 LIKE。

## 4. 英语

### english_articles

`id, title, level, category, content, word_count, difficulty, estimated_minutes, source, source_key,
source_name, source_category, source_url, normalized_source_url, external_id, published_at,
source_updated_at, image_url, audio_url, author, summary, fetched_at, rights_note, content_hash,
language, quality_score, has_audio, license_type, attribution, processing_status, fetch_status,
retry_count, last_error, created_time, questions_json, vocabulary_json, raw_json, 公共字段`

### english_learning_records

`id, user_id, article_id(FK 可空), record_date, reading_time_seconds, summary, score, analysis_id,
new_words_json, completion_status, reading_status, started_at, completed_at, 公共字段`

### english_highlights / english_notes

- highlights：`id, user_id, article_id(FK 可空), selected_text, block_id, start_offset, end_offset, color, prefix, suffix, note, 公共字段`
- notes：`id, user_id, article_id(FK 可空), quote, content, block_id, start_offset, end_offset, selected_text, prefix, suffix, highlight_id, 公共字段`

### english_vocabulary / vocabulary_occurrences / vocabulary_review_state

- vocabulary：`id, user_id, normalized_word, display_word, definition, phonetic, part_of_speech,
  selected_meanings_json, lemma, source_article_id, source_article_title, source_sentence, notes,
  mastery_level, review_stage, review_count, correct_count, incorrect_count, encounter_count,
  last_reviewed_at, next_review_at, status, frequency_rank, tags_json, metadata_json(含 reviewLogs), 公共字段`
  唯一索引：`(user_id, normalized_word) WHERE deleted_at IS NULL`
- occurrences：`id, vocabulary_id(FK CASCADE), article_id, article_title, source_sentence, created_at`
- review_state：`vocabulary_id(PK FK CASCADE), due_at, difficulty, stability, retrievability,
  review_count, lapse_count, scheduler_version, updated_at`（FSRS 字段预留，不实现算法）

### english_ai_analysis

`id, user_id, record_id(FK 可空), article_id, provider, score, content_score, grammar_score,
vocabulary_score, structure_score, mistakes_json, suggestions_json, improved_summary,
weak_points_json, 公共字段`

## 5. 训记与训练摘要

### workout_imports

`id, user_id, source, share_url, status, parser, parser_version, error, raw_json, workout_id(FK 可空), 公共字段`

### workouts

`id, user_id, source, source_id, name, occurred_at, local_date, duration_seconds, exercise_count,
set_count, planned_set_count, volume_kg, calories_kcal, status, raw_json, 公共字段`

### workout_exercises / workout_sets

- exercises：`id, workout_id(FK CASCADE), name, sort_order, planned_sets, completed_sets`
- sets：`id, exercise_id(FK CASCADE), set_number, weight_kg, reps, completed`

### training_notes

`id, user_id, title, content, workout_id(FK 可空), source, note_date, 公共字段`

## 6. 保留的 KV / 运行配置表

`settings`、`ai_settings`、`translation_settings`、`english_preferences`、`english_sources`、
`english_sync_tasks`、`import_uploads` 继续使用 JSON/KV（高频统计不依赖这些字段）。
