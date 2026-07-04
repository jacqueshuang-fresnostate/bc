-- 旧默认值会让开盘补期和封盘检查最多等待 60 秒，线上仍保持旧配置的库需要跟随新默认值。
UPDATE draw_scheduler_config
SET interval_seconds = 2,
    updated_at = now()
WHERE id = 'default'
  AND interval_seconds = 60;

-- 提前保留多个未来期号，降低调度短暂抖动或服务重启后下一期生成过晚的概率。
UPDATE draw_scheduler_config
SET future_issue_count = 3,
    updated_at = now()
WHERE id = 'default'
  AND future_issue_count = 1;
