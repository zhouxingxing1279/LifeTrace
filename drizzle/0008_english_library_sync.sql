ALTER TABLE `english_articles` ADD `source_key` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `source_name` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `source_category` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `external_id` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `source_url` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `normalized_source_url` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `title` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `summary` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `content` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `author` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `published_at` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `source_updated_at` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `fetched_at` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `created_at` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `content_hash` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `word_count` integer;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `language` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `cefr_level` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `estimated_reading_minutes` integer;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `quality_score` real;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `audio_url` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `image_url` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `has_audio` integer;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `license_type` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `attribution` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `processing_status` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `fetch_status` text;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `retry_count` integer DEFAULT 0;
--> statement-breakpoint
ALTER TABLE `english_articles` ADD `last_error` text;
--> statement-breakpoint
CREATE UNIQUE INDEX `english_articles_source_external_unique`
  ON `english_articles` (`source_key`, `external_id`)
  WHERE `source_key` IS NOT NULL AND `external_id` IS NOT NULL;
--> statement-breakpoint
CREATE UNIQUE INDEX `english_articles_source_url_unique`
  ON `english_articles` (`normalized_source_url`)
  WHERE `normalized_source_url` IS NOT NULL;
--> statement-breakpoint
CREATE INDEX `english_articles_content_hash_idx` ON `english_articles` (`content_hash`);
--> statement-breakpoint
CREATE INDEX `english_articles_processing_status_idx` ON `english_articles` (`processing_status`);
--> statement-breakpoint
CREATE TABLE `english_content_sources` (
  `id` text PRIMARY KEY NOT NULL,
  `source_key` text NOT NULL,
  `source_name` text NOT NULL,
  `source_type` text NOT NULL,
  `source_url` text NOT NULL,
  `category` text NOT NULL,
  `enabled` integer DEFAULT 1 NOT NULL,
  `sync_interval` integer DEFAULT 86400 NOT NULL,
  `initial_fetch_limit` integer DEFAULT 100 NOT NULL,
  `recent_scan_limit` integer DEFAULT 30 NOT NULL,
  `overlap_days` integer DEFAULT 14 NOT NULL,
  `request_interval_ms` integer DEFAULT 1000 NOT NULL,
  `last_sync_at` text,
  `last_success_at` text,
  `last_new_article_at` text,
  `latest_external_published_at` text,
  `sync_cursor` text,
  `consecutive_failures` integer DEFAULT 0 NOT NULL,
  `status` text DEFAULT 'active' NOT NULL,
  `last_error` text,
  `created_at` text NOT NULL,
  `updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `english_content_sources_source_key_unique` ON `english_content_sources` (`source_key`);
--> statement-breakpoint
CREATE TABLE `english_library_state` (
  `id` text PRIMARY KEY NOT NULL,
  `initialization_status` text DEFAULT 'not_started' NOT NULL,
  `initialized_at` text,
  `initial_article_count` integer DEFAULT 0 NOT NULL,
  `target_article_count` integer DEFAULT 500 NOT NULL,
  `current_source_key` text,
  `last_error` text,
  `created_at` text NOT NULL,
  `updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `english_sync_tasks` (
  `task_id` text PRIMARY KEY NOT NULL,
  `task_type` text NOT NULL,
  `source_key` text,
  `requested_limit` integer,
  `status` text NOT NULL,
  `started_at` text,
  `finished_at` text,
  `total_count` integer DEFAULT 0 NOT NULL,
  `success_count` integer DEFAULT 0 NOT NULL,
  `inserted_count` integer DEFAULT 0 NOT NULL,
  `updated_count` integer DEFAULT 0 NOT NULL,
  `skipped_count` integer DEFAULT 0 NOT NULL,
  `failed_count` integer DEFAULT 0 NOT NULL,
  `current_article` text,
  `progress` real DEFAULT 0 NOT NULL,
  `last_error` text,
  `created_at` text NOT NULL,
  `updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `english_sync_tasks_status_idx` ON `english_sync_tasks` (`status`, `created_at`);
--> statement-breakpoint
CREATE UNIQUE INDEX `english_sync_tasks_single_running`
  ON `english_sync_tasks` ((1))
  WHERE `status` IN ('PENDING', 'RUNNING');
--> statement-breakpoint
CREATE TABLE `english_sync_logs` (
  `id` text PRIMARY KEY NOT NULL,
  `task_id` text NOT NULL,
  `source_key` text,
  `level` text NOT NULL,
  `event` text NOT NULL,
  `request_url` text,
  `message` text NOT NULL,
  `retry_count` integer DEFAULT 0 NOT NULL,
  `duration_ms` integer,
  `details_json` text,
  `created_at` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `english_sync_logs_task_idx` ON `english_sync_logs` (`task_id`, `created_at`);
--> statement-breakpoint
CREATE TABLE `english_processing_queue` (
  `id` text PRIMARY KEY NOT NULL,
  `article_id` text NOT NULL,
  `job_type` text NOT NULL,
  `status` text DEFAULT 'PENDING' NOT NULL,
  `retry_count` integer DEFAULT 0 NOT NULL,
  `last_error` text,
  `available_at` text NOT NULL,
  `created_at` text NOT NULL,
  `updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `english_processing_queue_article_job_unique`
  ON `english_processing_queue` (`article_id`, `job_type`);
