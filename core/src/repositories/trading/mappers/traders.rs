use crate::{entity, time::dt_to_ts};

use super::super::{records::traders::TraderRecord, values::decimal_to_f64};

pub(in crate::repositories::trading) fn map_trader(row: entity::traders::Model) -> TraderRecord {
    TraderRecord {
        id: row.id,
        name: row.name,
        ai_model_id: row.ai_model_id,
        exchange_id: row.exchange_id,
        strategy_id: row.strategy_id,
        initial_balance: decimal_to_f64(&row.initial_balance),
        scan_interval_minutes: i64::from(row.scan_interval_minutes),
        is_running: i64::from(row.is_running),
        is_cross_margin: i64::from(row.is_cross_margin),
        use_ai500: i64::from(row.use_ai500),
        use_oi_top: i64::from(row.use_oi_top),
        custom_prompt: row.custom_prompt,
        override_base_prompt: i64::from(row.override_base_prompt),
        system_prompt_template: row.system_prompt_template,
        created_at: dt_to_ts(row.created_at),
        updated_at: dt_to_ts(row.updated_at),
    }
}
