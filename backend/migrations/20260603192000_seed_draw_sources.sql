INSERT INTO draw_sources (id, name, provider, lot_code, endpoint, reusable_for_lottery_ids)
VALUES
    ('api68-fc3d', 'API68 福彩 3D/排列 3', 'api68', '10041', 'https://api.api68.com/QuanGuoCai/getLotteryInfoList.do', '["fc3d","pl3"]'::jsonb),
    ('api68-au5', 'API68 澳洲 5 分彩', 'api68', '10010', 'https://api.api68.com/CQShiCai/getBaseCQShiCaiList.do', '["au5"]'::jsonb),
    ('kj-txffc', 'KJAPI腾讯分分彩', 'kjApi', 'txffc', 'https://kjapi.net/hall/hallajax/getLotteryInfo', '["txffc"]'::jsonb)
ON CONFLICT (id) DO NOTHING;
