-- 将体彩排列 3 从福彩 3D 共享开奖源拆出，避免两个彩种继续复用同一个 API68 lotCode。
UPDATE draw_sources
SET reusable_for_lottery_ids = COALESCE(
        (
            SELECT jsonb_agg(item.value ORDER BY item.ordinality)
            FROM jsonb_array_elements_text(draw_sources.reusable_for_lottery_ids)
                WITH ORDINALITY AS item(value, ordinality)
            WHERE item.value <> 'pl3'
        ),
        '[]'::jsonb
    ),
    updated_at = now()
WHERE id = 'api68-fc3d'
  AND reusable_for_lottery_ids ? 'pl3';

-- 兼容旧默认名称，已被运营手动改名的彩种不覆盖。
UPDATE lotteries
SET name = '体彩排列3',
    updated_at = now()
WHERE id = 'pl3'
  AND name IN ('排列 3', '排列3');

-- 新增体彩排列 3 独立开奖源；如果已有其它来源绑定 pl3，则不抢占运营配置。
INSERT INTO draw_sources (id, name, provider, lot_code, endpoint, reusable_for_lottery_ids)
SELECT
    'api68-pl3',
    'API68 体彩排列3',
    'api68',
    '10043',
    'https://api.api68.com/QuanGuoCai/getLotteryInfo1.do',
    '["pl3"]'::jsonb
WHERE NOT EXISTS (
    SELECT 1
    FROM draw_sources existing
    CROSS JOIN LATERAL jsonb_array_elements_text(existing.reusable_for_lottery_ids) AS item(value)
    WHERE item.value = 'pl3'
)
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    provider = EXCLUDED.provider,
    lot_code = EXCLUDED.lot_code,
    endpoint = EXCLUDED.endpoint,
    reusable_for_lottery_ids = EXCLUDED.reusable_for_lottery_ids,
    updated_at = now()
WHERE NOT EXISTS (
    SELECT 1
    FROM draw_sources existing
    CROSS JOIN LATERAL jsonb_array_elements_text(existing.reusable_for_lottery_ids) AS item(value)
    WHERE item.value = 'pl3'
      AND existing.id <> 'api68-pl3'
);

-- 新增体彩排列 5 独立开奖源；pl5 彩种完整默认配置由后端启动 seed 补齐。
INSERT INTO draw_sources (id, name, provider, lot_code, endpoint, reusable_for_lottery_ids)
SELECT
    'api68-pl5',
    'API68 体彩排列5',
    'api68',
    '10044',
    'https://api.api68.com/QuanGuoCai/getLotteryInfo.do',
    '["pl5"]'::jsonb
WHERE NOT EXISTS (
    SELECT 1
    FROM draw_sources existing
    CROSS JOIN LATERAL jsonb_array_elements_text(existing.reusable_for_lottery_ids) AS item(value)
    WHERE item.value = 'pl5'
)
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    provider = EXCLUDED.provider,
    lot_code = EXCLUDED.lot_code,
    endpoint = EXCLUDED.endpoint,
    reusable_for_lottery_ids = EXCLUDED.reusable_for_lottery_ids,
    updated_at = now()
WHERE NOT EXISTS (
    SELECT 1
    FROM draw_sources existing
    CROSS JOIN LATERAL jsonb_array_elements_text(existing.reusable_for_lottery_ids) AS item(value)
    WHERE item.value = 'pl5'
      AND existing.id <> 'api68-pl5'
);
