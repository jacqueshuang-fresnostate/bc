ALTER TABLE draw_scheduler_runs
ADD COLUMN skipped_issues JSONB NOT NULL DEFAULT '[]'::jsonb,
ADD COLUMN skipped_lotteries JSONB NOT NULL DEFAULT '[]'::jsonb;
