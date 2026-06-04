ALTER TABLE lotteries
ADD COLUMN logo_url TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN lotteries.logo_url IS '彩种LOGO图片链接';

COMMENT ON TABLE lotteries IS '彩种配置表，保存每个彩种的基础参数、玩法与调度能力';
COMMENT ON COLUMN lotteries.id IS '彩种唯一标识符';
COMMENT ON COLUMN lotteries.name IS '彩种展示名称';
COMMENT ON COLUMN lotteries.logo_url IS '彩种 LOGO 链接地址';
COMMENT ON COLUMN lotteries.category IS '彩种分类：regional（地方彩种）、overseas（海外彩种）、welfare（福利彩种）、other（其他）';
COMMENT ON COLUMN lotteries.number_type IS '开奖号码位数类型，支持 threeDigit/fiveDigit';
COMMENT ON COLUMN lotteries.draw_mode IS '开奖模式：platform（平台开奖）、api（采集开奖）、manual（手工开奖）';
COMMENT ON COLUMN lotteries.schedule IS '开奖号码生成与开奖时序配置（JSON）';
COMMENT ON COLUMN lotteries.sale_enabled IS '是否对外开放销售';
COMMENT ON COLUMN lotteries.group_buy IS '合买配置（例如是否允许、相关阈值）';
COMMENT ON COLUMN lotteries.play_categories IS '玩法分类配置（JSON）';
COMMENT ON COLUMN lotteries.play_configs IS '玩法赔率与选号约束的动态配置';
COMMENT ON COLUMN lotteries.created_at IS '记录创建时间';
COMMENT ON COLUMN lotteries.updated_at IS '记录更新时间';
