-- 将平台开奖的普通周期彩种迁移到自然时间节点周期，保证 00:00 开盘后按整日节点对齐。
UPDATE lotteries
SET
    schedule = jsonb_build_object(
        'timeNode',
        jsonb_build_object(
            'intervalSeconds',
            (schedule #>> '{periodic,intervalSeconds}')::integer,
            'startTime',
            '00:00:00'
        )
    ),
    updated_at = now()
WHERE draw_mode = 'platform'
  AND schedule ? 'periodic'
  AND (schedule #>> '{periodic,intervalSeconds}') ~ '^[0-9]+$'
  AND (schedule #>> '{periodic,intervalSeconds}')::integer > 0
  AND 86400 % (schedule #>> '{periodic,intervalSeconds}')::integer = 0;
