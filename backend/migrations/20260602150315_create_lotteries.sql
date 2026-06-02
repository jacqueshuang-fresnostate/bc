CREATE TABLE lotteries (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    number_type TEXT NOT NULL,
    draw_mode TEXT NOT NULL,
    schedule JSONB NOT NULL,
    sale_enabled BOOLEAN NOT NULL,
    group_buy JSONB NOT NULL,
    play_categories JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT lotteries_number_type_check CHECK (number_type IN ('threeDigit', 'fiveDigit')),
    CONSTRAINT lotteries_draw_mode_check CHECK (draw_mode IN ('platform', 'api', 'manual'))
);

CREATE INDEX lotteries_sale_enabled_idx ON lotteries (sale_enabled);
