CREATE TEMP TABLE removed_api68_fast_three_lotteries (
    id TEXT PRIMARY KEY
) ON COMMIT DROP;

INSERT INTO removed_api68_fast_three_lotteries (id)
VALUES
    ('ahk3'),
    ('bjk3'),
    ('fjk3'),
    ('gxk3'),
    ('hebk3'),
    ('hubk3'),
    ('jlk3'),
    ('jsk3'),
    ('nmgk3');

CREATE TEMP TABLE affected_api68_fast_three_draw_sources (
    id TEXT PRIMARY KEY
) ON COMMIT DROP;

INSERT INTO affected_api68_fast_three_draw_sources (id)
SELECT draw_sources.id
FROM draw_sources
WHERE EXISTS (
    SELECT 1
    FROM jsonb_array_elements_text(draw_sources.reusable_for_lottery_ids) AS item(value)
    WHERE item.value IN (SELECT id FROM removed_api68_fast_three_lotteries)
);

DELETE FROM draw_controls
WHERE lottery_id IN (SELECT id FROM removed_api68_fast_three_lotteries);

DELETE FROM draw_issues
WHERE lottery_id IN (SELECT id FROM removed_api68_fast_three_lotteries);

DELETE FROM robot_lottery_bindings
WHERE lottery_id IN (SELECT id FROM removed_api68_fast_three_lotteries);

DELETE FROM group_buy_participants
WHERE plan_id IN (
    SELECT id
    FROM group_buy_plans
    WHERE lottery_id IN (SELECT id FROM removed_api68_fast_three_lotteries)
);

DELETE FROM group_buy_plans
WHERE lottery_id IN (SELECT id FROM removed_api68_fast_three_lotteries);

UPDATE draw_sources
SET reusable_for_lottery_ids = COALESCE(
    (
        SELECT jsonb_agg(item.value ORDER BY item.ordinality)
        FROM jsonb_array_elements_text(draw_sources.reusable_for_lottery_ids) WITH ORDINALITY AS item(value, ordinality)
        WHERE item.value NOT IN (SELECT id FROM removed_api68_fast_three_lotteries)
    ),
    '[]'::jsonb
)
WHERE EXISTS (
    SELECT 1
    FROM jsonb_array_elements_text(draw_sources.reusable_for_lottery_ids) AS item(value)
    WHERE item.value IN (SELECT id FROM removed_api68_fast_three_lotteries)
);

DELETE FROM draw_sources
WHERE id IN (
    SELECT 'api68-' || id
    FROM removed_api68_fast_three_lotteries
)
OR (
    id IN (SELECT id FROM affected_api68_fast_three_draw_sources)
    AND jsonb_array_length(reusable_for_lottery_ids) = 0
);

DELETE FROM lotteries
WHERE id IN (SELECT id FROM removed_api68_fast_three_lotteries);

DROP TABLE removed_api68_fast_three_lotteries;
DROP TABLE affected_api68_fast_three_draw_sources;
