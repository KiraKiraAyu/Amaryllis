use crate::{entity, time::dt_to_ts};

use super::super::{records::positions::TraderPositionRecord, values::decimal_to_f64};

pub(in crate::repositories::trading) fn map_position(
    row: entity::trader_positions::Model,
) -> TraderPositionRecord {
    TraderPositionRecord {
        id: row.id,
        trader_id: row.trader_id,
        symbol: row.symbol,
        side: row.side,
        quantity: decimal_to_f64(&row.quantity),
        entry_price: decimal_to_f64(&row.entry_price),
        mark_price: decimal_to_f64(&row.mark_price),
        liquidation_price: decimal_to_f64(&row.liquidation_price),
        leverage: row.leverage,
        margin_mode: row.margin_mode,
        unrealized_pnl: decimal_to_f64(&row.unrealized_pnl),
        realized_pnl: decimal_to_f64(&row.realized_pnl),
        status: row.status,
        opened_at: dt_to_ts(row.opened_at),
        closed_at: row.closed_at.map(dt_to_ts),
        updated_at: dt_to_ts(row.updated_at),
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;

    #[test]
    fn map_position_preserves_trader_id_and_full_position_fields() {
        let row = crate::entity::trader_positions::Model {
            id: "position_1".to_string(),
            trader_id: "trader_1".to_string(),
            symbol: "BTCUSDT".to_string(),
            side: "LONG".to_string(),
            quantity: Decimal::new(125, 2),
            entry_price: Decimal::new(2565, 1),
            mark_price: Decimal::new(26075, 2),
            liquidation_price: Decimal::new(128, 0),
            leverage: 5,
            margin_mode: "cross".to_string(),
            unrealized_pnl: Decimal::new(125, 1),
            realized_pnl: Decimal::new(-25, 1),
            status: "open".to_string(),
            opened_at: crate::time::ts_to_dt(1_700_000_000),
            closed_at: Some(crate::time::ts_to_dt(1_700_000_600)),
            created_at: crate::time::ts_to_dt(1_699_999_900),
            updated_at: crate::time::ts_to_dt(1_700_000_900),
        };

        let record = map_position(row);

        assert_eq!(record.id, "position_1");
        assert_eq!(record.trader_id, "trader_1");
        assert_eq!(record.symbol, "BTCUSDT");
        assert_eq!(record.side, "LONG");
        assert_eq!(record.quantity, 1.25);
        assert_eq!(record.entry_price, 256.5);
        assert_eq!(record.mark_price, 260.75);
        assert_eq!(record.liquidation_price, 128.0);
        assert_eq!(record.leverage, 5);
        assert_eq!(record.margin_mode, "cross");
        assert_eq!(record.unrealized_pnl, 12.5);
        assert_eq!(record.realized_pnl, -2.5);
        assert_eq!(record.status, "open");
        assert_eq!(record.opened_at, 1_700_000_000);
        assert_eq!(record.closed_at, Some(1_700_000_600));
        assert_eq!(record.updated_at, 1_700_000_900);
    }
}
