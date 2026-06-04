CREATE TABLE lottery_categories (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO lottery_categories (code, name, sort_order)
VALUES
    ('regional', '地方彩种', 0),
    ('overseas', '海外彩种', 1),
    ('welfare', '福利彩种', 2),
    ('other', '其他', 3)
ON CONFLICT (code) DO NOTHING;

ALTER TABLE lotteries DROP CONSTRAINT IF EXISTS lotteries_category_check;

COMMENT ON COLUMN lotteries.category IS '彩种分类编码，如 regional/overseas/welfare/other，支持扩展为自定义编码';
