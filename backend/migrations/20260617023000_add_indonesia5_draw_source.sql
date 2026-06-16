-- 新增印尼5分彩开奖源，已有同 ID 配置时保留后台自定义内容。
INSERT INTO draw_sources (id, name, provider, lot_code, endpoint, reusable_for_lottery_ids)
VALUES (
    'indonesia-id5',
    '印尼开奖 印尼5分彩',
    'indonesiaLottery',
    'IDFFC5',
    'https://draw.indonesia-lottery.org/others/draw.php',
    '["id5"]'::jsonb
)
ON CONFLICT (id) DO NOTHING;

COMMENT ON COLUMN draw_sources.provider IS '开奖方类型（如 api68/kjApi/bbKaijiang/indonesiaLottery）';
COMMENT ON COLUMN draw_sources.lot_code IS '开奖源编码参数，如 lotCode/lotKey/gameCodeList；部分来源仅作展示编码';
