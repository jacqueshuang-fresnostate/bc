-- 新增河内5分彩 BB 开奖源，已有同 ID 配置时保留后台自定义内容。
INSERT INTO draw_sources (id, name, provider, lot_code, endpoint, reusable_for_lottery_ids)
VALUES (
    'bb-hn5',
    'BB开奖 河内5分彩',
    'bbKaijiang',
    'VIFFC5',
    'https://www.bbkaijiang.com/api/st-lottery-open/open-result/list-newest-result',
    '["hn5"]'::jsonb
)
ON CONFLICT (id) DO NOTHING;

COMMENT ON COLUMN draw_sources.provider IS '开奖方类型（如 api68/kjApi/bbKaijiang）';
COMMENT ON COLUMN draw_sources.lot_code IS '开奖源期号参数，如 lotCode/lotKey/gameCodeList';
