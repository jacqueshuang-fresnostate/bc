ALTER TABLE lotteries
ADD COLUMN play_configs JSONB NOT NULL DEFAULT '[]'::jsonb;
