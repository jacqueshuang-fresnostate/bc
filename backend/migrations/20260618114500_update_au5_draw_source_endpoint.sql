UPDATE draw_sources
SET endpoint = 'https://api.api68.com/CQShiCai/getBaseCQShiCai.do',
    name = CASE
        WHEN name IN ('API68 澳洲 5 分彩', 'API68 澳洲5分彩') THEN 'API68 澳洲幸运5'
        ELSE name
    END,
    updated_at = now()
WHERE id = 'api68-au5'
  AND provider = 'api68'
  AND lot_code = '10010';

COMMENT ON COLUMN draw_sources.endpoint IS '开奖源基础接口地址；澳洲幸运5默认使用 https://api.api68.com/CQShiCai/getBaseCQShiCai.do';
