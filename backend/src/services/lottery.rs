use std::collections::BTreeMap;

use crate::{
    domain::lottery::{
        DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType, PlayCategory,
    },
    error::{ApiError, ApiResult},
};

#[derive(Debug, Clone)]
pub struct LotteryStore {
    lotteries: BTreeMap<String, LotteryKind>,
}

impl LotteryStore {
    pub fn seeded() -> Self {
        Self::from_lotteries(seed_lotteries())
    }

    fn from_lotteries(lotteries: Vec<LotteryKind>) -> Self {
        Self {
            lotteries: lotteries
                .into_iter()
                .map(|lottery| (lottery.id.clone(), lottery))
                .collect(),
        }
    }

    pub fn list(&self) -> Vec<LotteryKind> {
        self.lotteries.values().cloned().collect()
    }

    pub fn get(&self, id: &str) -> ApiResult<LotteryKind> {
        self.lotteries
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

    pub fn create(&mut self, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        validate_lottery(&lottery)?;

        if self.lotteries.contains_key(&lottery.id) {
            return Err(ApiError::Conflict(format!(
                "lottery `{}` already exists",
                lottery.id
            )));
        }

        self.lotteries.insert(lottery.id.clone(), lottery.clone());
        Ok(lottery)
    }

    pub fn update(&mut self, id: &str, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        validate_lottery(&lottery)?;

        if id != lottery.id {
            return Err(ApiError::BadRequest(
                "path id must match lottery id".to_string(),
            ));
        }

        if !self.lotteries.contains_key(id) {
            return Err(ApiError::NotFound(format!("lottery `{id}` not found")));
        }

        self.lotteries.insert(id.to_string(), lottery.clone());
        Ok(lottery)
    }

    pub fn delete(&mut self, id: &str) -> ApiResult<LotteryKind> {
        self.lotteries
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

    pub fn set_sale_enabled(&mut self, id: &str, sale_enabled: bool) -> ApiResult<LotteryKind> {
        let lottery = self
            .lotteries
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))?;

        lottery.sale_enabled = sale_enabled;
        Ok(lottery.clone())
    }
}

pub fn seed_lotteries() -> Vec<LotteryKind> {
    vec![
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: true,
            group_buy: group_buy_config(),
            play_categories: vec![
                PlayCategory::Direct,
                PlayCategory::GroupThree,
                PlayCategory::GroupSix,
            ],
        },
        LotteryKind {
            id: "pl3".to_string(),
            name: "排列 3".to_string(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: true,
            group_buy: group_buy_config(),
            play_categories: vec![
                PlayCategory::Direct,
                PlayCategory::GroupThree,
                PlayCategory::GroupSix,
            ],
        },
        LotteryKind {
            id: "ssc60".to_string(),
            name: "60 秒时时彩".to_string(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Platform,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: true,
            group_buy: group_buy_config(),
            play_categories: vec![
                PlayCategory::Direct,
                PlayCategory::DirectCombination,
                PlayCategory::GroupThree,
                PlayCategory::GroupSix,
                PlayCategory::BigSmallOddEven,
            ],
        },
        LotteryKind {
            id: "manual-test".to_string(),
            name: "指定号码测试彩".to_string(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Manual,
            schedule: DrawSchedule::Weekly {
                weekdays: vec!["Tuesday".to_string(), "Thursday".to_string()],
                time: "21:00:00".to_string(),
            },
            sale_enabled: false,
            group_buy: GroupBuyConfig {
                enabled: false,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1_000,
            },
            play_categories: vec![PlayCategory::Direct],
        },
    ]
}

fn validate_lottery(lottery: &LotteryKind) -> ApiResult<()> {
    if lottery.id.trim().is_empty() {
        return Err(ApiError::BadRequest("lottery id is required".to_string()));
    }

    if lottery.name.trim().is_empty() {
        return Err(ApiError::BadRequest("lottery name is required".to_string()));
    }

    if lottery.play_categories.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one play category is required".to_string(),
        ));
    }

    match &lottery.schedule {
        DrawSchedule::Periodic { interval_seconds } if *interval_seconds == 0 => {
            return Err(ApiError::BadRequest(
                "periodic interval must be greater than zero".to_string(),
            ));
        }
        DrawSchedule::Daily { time } if time.trim().is_empty() => {
            return Err(ApiError::BadRequest(
                "daily draw time is required".to_string(),
            ));
        }
        DrawSchedule::Weekly { weekdays, time } => {
            if weekdays.is_empty() {
                return Err(ApiError::BadRequest(
                    "weekly draw weekdays are required".to_string(),
                ));
            }
            if time.trim().is_empty() {
                return Err(ApiError::BadRequest(
                    "weekly draw time is required".to_string(),
                ));
            }
        }
        _ => {}
    }

    if lottery.group_buy.min_share_amount_minor <= 0 {
        return Err(ApiError::BadRequest(
            "group buy min share amount must be greater than zero".to_string(),
        ));
    }

    if lottery.group_buy.initiator_min_percent > 100 {
        return Err(ApiError::BadRequest(
            "initiator min percent must be between 0 and 100".to_string(),
        ));
    }

    if lottery.group_buy.participant_min_amount_minor <= 0 {
        return Err(ApiError::BadRequest(
            "participant min amount must be greater than zero".to_string(),
        ));
    }

    Ok(())
}

fn group_buy_config() -> GroupBuyConfig {
    GroupBuyConfig {
        enabled: true,
        min_share_amount_minor: 100,
        initiator_min_percent: 10,
        participant_min_amount_minor: 1_000,
    }
}

#[cfg(test)]
mod tests {
    use super::{seed_lotteries, LotteryStore};
    use crate::domain::lottery::{DrawSchedule, LotteryKind};

    #[test]
    fn store_creates_and_lists_lottery() {
        let mut store = LotteryStore::seeded();
        let mut lottery = seed_lotteries()
            .into_iter()
            .find(|item| item.id == "fc3d")
            .expect("seed lottery exists");
        lottery.id = "new3d".to_string();
        lottery.name = "新 3D".to_string();

        store.create(lottery).expect("lottery can be created");

        assert!(store.list().iter().any(|item| item.id == "new3d"));
    }

    #[test]
    fn store_rejects_duplicate_id() {
        let mut store = LotteryStore::seeded();
        let lottery = store.get("fc3d").expect("seed lottery exists");

        let error = store.create(lottery).expect_err("duplicate id rejected");

        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn store_rejects_invalid_periodic_schedule() {
        let mut store = LotteryStore::seeded();
        let mut lottery: LotteryKind = store.get("ssc60").expect("seed lottery exists");
        lottery.id = "bad-period".to_string();
        lottery.schedule = DrawSchedule::Periodic {
            interval_seconds: 0,
        };

        let error = store
            .create(lottery)
            .expect_err("zero interval should be rejected");

        assert!(error.to_string().contains("greater than zero"));
    }

    #[test]
    fn store_toggles_sale_status() {
        let mut store = LotteryStore::seeded();

        let updated = store
            .set_sale_enabled("fc3d", false)
            .expect("sale status can be changed");

        assert!(!updated.sale_enabled);
    }
}
