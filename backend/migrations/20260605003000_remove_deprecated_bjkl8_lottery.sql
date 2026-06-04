CREATE TEMP TABLE removed_api68_bjkl8_lottery (
    id TEXT PRIMARY KEY
) ON COMMIT DROP;

INSERT INTO removed_api68_bjkl8_lottery (id)
VALUES
    ('bjkl8');

CREATE TEMP TABLE affected_api68_bjkl8_draw_sources (
    id TEXT PRIMARY KEY
) ON COMMIT DROP;

INSERT INTO affected_api68_bjkl8_draw_sources (id)
SELECT draw_sources.id
FROM draw_sources
WHERE EXISTS (
    SELECT 1
    FROM jsonb_array_elements_text(draw_sources.reusable_for_lottery_ids) AS item(value)
    WHERE item.value IN (SELECT id FROM removed_api68_bjkl8_lottery)
);

DELETE FROM draw_controls
WHERE lottery_id IN (SELECT id FROM removed_api68_bjkl8_lottery);

DELETE FROM draw_issues
WHERE lottery_id IN (SELECT id FROM removed_api68_bjkl8_lottery);

DELETE FROM robot_lottery_bindings
WHERE lottery_id IN (SELECT id FROM removed_api68_bjkl8_lottery);

DELETE FROM group_buy_participants
WHERE plan_id IN (
    SELECT id
    FROM group_buy_plans
    WHERE lottery_id IN (SELECT id FROM removed_api68_bjkl8_lottery)
);

DELETE FROM group_buy_plans
WHERE lottery_id IN (SELECT id FROM removed_api68_bjkl8_lottery);

UPDATE draw_sources
SET reusable_for_lottery_ids = COALESCE(
    (
        SELECT jsonb_agg(item.value ORDER BY item.ordinality)
        FROM jsonb_array_elements_text(draw_sources.reusable_for_lottery_ids) WITH ORDINALITY AS item(value, ordinality)
        WHERE item.value NOT IN (SELECT id FROM removed_api68_bjkl8_lottery)
    ),
    '[]'::jsonb
)
WHERE EXISTS (
    SELECT 1
    FROM jsonb_array_elements_text(draw_sources.reusable_for_lottery_ids) AS item(value)
    WHERE item.value IN (SELECT id FROM removed_api68_bjkl8_lottery)
);

DELETE FROM draw_sources
WHERE id = 'api68-bjkl8'
OR (
    id IN (SELECT id FROM affected_api68_bjkl8_draw_sources)
    AND jsonb_array_length(reusable_for_lottery_ids) = 0
);

DELETE FROM lotteries
WHERE id IN (SELECT id FROM removed_api68_bjkl8_lottery);

DROP TABLE affected_api68_bjkl8_draw_sources;
DROP TABLE removed_api68_bjkl8_lottery;
