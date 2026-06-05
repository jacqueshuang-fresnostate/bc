ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS order_source TEXT NOT NULL DEFAULT 'direct';

COMMENT ON COLUMN orders.order_source IS '订单来源：direct 表示独立下单，groupBuy 表示合买满单生成';

UPDATE orders
SET order_source = 'direct'
WHERE order_source IS NULL OR trim(order_source) = '';
