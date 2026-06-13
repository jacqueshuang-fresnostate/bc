ALTER TABLE lotteries
ADD COLUMN draw_control_enabled BOOLEAN NOT NULL DEFAULT true;

COMMENT ON COLUMN lotteries.draw_control_enabled IS '是否允许后台控制该彩种开奖号码，关闭后管理端不展示控制入口且接口不允许启用控制';
