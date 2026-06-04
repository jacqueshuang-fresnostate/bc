CREATE TABLE advertisements (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    image_url TEXT NOT NULL,
    link_url TEXT,
    placement TEXT NOT NULL,
    status TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    start_at TEXT,
    end_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT advertisements_placement_check CHECK (placement IN ('mobileCarousel')),
    CONSTRAINT advertisements_status_check CHECK (status IN ('enabled', 'disabled')),
    CONSTRAINT advertisements_sort_order_check CHECK (sort_order >= 0)
);

CREATE INDEX advertisements_mobile_carousel_idx
    ON advertisements (placement, status, sort_order, id);

COMMENT ON TABLE advertisements IS '广告管理配置表，用于后台维护手机端轮播广告';
COMMENT ON COLUMN advertisements.id IS '广告 ID，后台可手动填写，留空创建时由后端自动生成';
COMMENT ON COLUMN advertisements.title IS '广告标题，用于后台识别和手机端展示';
COMMENT ON COLUMN advertisements.image_url IS '广告图片链接，手机端轮播图展示地址';
COMMENT ON COLUMN advertisements.link_url IS '广告跳转链接，可为空';
COMMENT ON COLUMN advertisements.placement IS '广告位置，目前支持 mobileCarousel（手机端轮播）';
COMMENT ON COLUMN advertisements.status IS '广告状态：enabled（启用）或 disabled（停用）';
COMMENT ON COLUMN advertisements.sort_order IS '展示排序值，越小越靠前';
COMMENT ON COLUMN advertisements.start_at IS '广告开始展示时间，格式为 YYYY-MM-DD HH:MM:SS，可为空';
COMMENT ON COLUMN advertisements.end_at IS '广告结束展示时间，格式为 YYYY-MM-DD HH:MM:SS，可为空';
COMMENT ON COLUMN advertisements.created_at IS '广告创建时间，格式为 YYYY-MM-DD HH:MM:SS';
COMMENT ON COLUMN advertisements.updated_at IS '广告更新时间，格式为 YYYY-MM-DD HH:MM:SS';
COMMENT ON CONSTRAINT advertisements_placement_check ON advertisements IS '限制广告位置枚举值';
COMMENT ON CONSTRAINT advertisements_status_check ON advertisements IS '限制广告状态枚举值';
COMMENT ON CONSTRAINT advertisements_sort_order_check ON advertisements IS '限制广告排序值不能为负数';
