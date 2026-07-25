CREATE TABLE `english_user_vocabulary` (
  `id` text PRIMARY KEY NOT NULL, `word` text NOT NULL, `normalized_word` text NOT NULL,
  `lemma` text NOT NULL, `dictionary_word_id` integer, `phonetic` text,
  `selected_meanings_json` text NOT NULL, `part_of_speech` text,
  `source_article_id` text, `source_article_title` text, `source_sentence` text, `notes` text,
  `mastery_level` integer DEFAULT 0 NOT NULL, `review_stage` integer DEFAULT 0 NOT NULL,
  `review_count` integer DEFAULT 0 NOT NULL, `correct_count` integer DEFAULT 0 NOT NULL,
  `incorrect_count` integer DEFAULT 0 NOT NULL, `encounter_count` integer DEFAULT 1 NOT NULL,
  `last_reviewed_at` text, `next_review_at` text, `status` text DEFAULT 'LEARNING' NOT NULL,
  `frequency_rank` integer, `tags_json` text DEFAULT '[]' NOT NULL,
  `created_at` text NOT NULL, `updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `english_user_vocabulary_lemma_unique` ON `english_user_vocabulary` (`lemma`);
--> statement-breakpoint
CREATE INDEX `english_user_vocabulary_review_idx` ON `english_user_vocabulary` (`status`, `next_review_at`);
--> statement-breakpoint
CREATE INDEX `english_user_vocabulary_created_idx` ON `english_user_vocabulary` (`created_at`);
--> statement-breakpoint
CREATE TABLE `english_vocabulary_occurrences` (
  `id` text PRIMARY KEY NOT NULL, `vocabulary_id` text NOT NULL, `article_id` text,
  `article_title` text, `source_sentence` text NOT NULL, `created_at` text NOT NULL,
  FOREIGN KEY (`vocabulary_id`) REFERENCES `english_user_vocabulary`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE UNIQUE INDEX `english_vocabulary_occurrence_unique`
  ON `english_vocabulary_occurrences` (`vocabulary_id`, `article_id`, `source_sentence`);
--> statement-breakpoint
CREATE INDEX `english_vocabulary_occurrence_word_idx` ON `english_vocabulary_occurrences` (`vocabulary_id`, `created_at`);
--> statement-breakpoint
CREATE TABLE `english_vocabulary_review_logs` (
  `id` text PRIMARY KEY NOT NULL, `vocabulary_id` text NOT NULL, `result` text NOT NULL,
  `stage_before` integer NOT NULL, `stage_after` integer NOT NULL, `reviewed_at` text NOT NULL,
  `next_review_at` text, `response_time_ms` integer,
  FOREIGN KEY (`vocabulary_id`) REFERENCES `english_user_vocabulary`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE INDEX `english_vocabulary_review_log_idx` ON `english_vocabulary_review_logs` (`vocabulary_id`, `reviewed_at`);
--> statement-breakpoint
CREATE TABLE `english_vocabulary_settings` (
  `id` text PRIMARY KEY NOT NULL, `data_json` text NOT NULL, `updated_at` text NOT NULL
);
--> statement-breakpoint
INSERT OR IGNORE INTO `english_user_vocabulary`
(`id`,`word`,`normalized_word`,`lemma`,`phonetic`,`selected_meanings_json`,`part_of_speech`,
 `source_article_id`,`source_sentence`,`mastery_level`,`review_stage`,`review_count`,
 `next_review_at`,`status`,`created_at`,`updated_at`)
SELECT id,
 json_extract(data_json,'$.word'),
 lower(json_extract(data_json,'$.word')),
 lower(json_extract(data_json,'$.word')),
 json_extract(data_json,'$.phonetic'),
 json_array(json_extract(data_json,'$.meaning')),
 '',
 json_extract(data_json,'$.sourceArticleId'),
 json_extract(data_json,'$.example'),
 coalesce(json_extract(data_json,'$.masterLevel'),0),
 coalesce(json_extract(data_json,'$.masterLevel'),0),
 coalesce(json_extract(data_json,'$.reviewCount'),0),
 json_extract(data_json,'$.nextReviewTime'),
 CASE WHEN coalesce(json_extract(data_json,'$.masterLevel'),0) >= 5 THEN 'MASTERED' ELSE 'LEARNING' END,
 json_extract(data_json,'$.createdAt'),
 json_extract(data_json,'$.updatedAt')
FROM `english_vocabulary`
WHERE json_extract(data_json,'$.word') IS NOT NULL;
