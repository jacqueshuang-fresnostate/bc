ALTER TABLE lotteries
    DROP CONSTRAINT IF EXISTS lotteries_number_type_check;

ALTER TABLE lotteries
    ADD CONSTRAINT lotteries_number_type_check
    CHECK (
        number_type = ANY (
            ARRAY[
                'threeDigit'::text,
                'fiveDigit'::text,
                'pk10'::text,
                'elevenFive'::text,
                'fastThree'::text,
                'luckTwenty'::text
            ]
        )
    );

COMMENT ON CONSTRAINT lotteries_number_type_check ON lotteries IS '限制彩种号码类型只能使用系统支持的枚举值';
