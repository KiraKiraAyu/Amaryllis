use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::{Stream, StreamExt, stream};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::contracts::trading::positions::PositionPayload;
use crate::state::AppState;

pub const REALTIME_CHANNEL_CAPACITY: usize = 512;
const SSE_CONNECTED_COMMENT: &str = "connected";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeEvent {
    PositionUpdate {
        trader_id: String,
        positions: Vec<PositionPayload>,
    },
    TradeExecution {
        trader_id: String,
        trade: serde_json::Value,
    },
    AiPrompt {
        trader_id: String,
        symbol: String,
        prompt: String,
        system_prompt: Option<String>,
    },
    AiDecision {
        trader_id: String,
        decision: serde_json::Value,
    },
    EngineStatus {
        trader_id: String,
        status: String,
        message: String,
    },
    EquitySnapshot {
        trader_id: String,
        equity: f64,
        available_cash: f64,
        unrealized_pnl: f64,
        ts: i64,
    },
    BacktestProgress {
        run_id: String,
        state: String,
        bar_index: usize,
        total_bars: usize,
        equity: f64,
        ts: i64,
    },
    DebateMessage {
        debate_id: String,
        round: i64,
        personality: String,
        content: String,
        vote: String,
    },
    DebateFinished {
        debate_id: String,
        status: String,
        final_decision: String,
        final_reasoning: String,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Clone, Debug)]
pub struct RealtimeHub {
    tx: broadcast::Sender<Arc<RealtimeEvent>>,
}

impl RealtimeHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(REALTIME_CHANNEL_CAPACITY);
        Self { tx }
    }

    pub fn publish(&self, event: RealtimeEvent) -> usize {
        self.tx.send(Arc::new(event)).unwrap_or(0)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<RealtimeEvent>> {
        self.tx.subscribe()
    }
}

impl Default for RealtimeHub {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE stream handler. Authentication is enforced by the route-level
/// auth middleware (token via `Authorization` header or `?token=` query).
pub async fn events_handler(
    State(app): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = app.realtime_hub.subscribe();

    debug!("sse: client connected");

    let event_stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = match serde_json::to_string(event.as_ref()) {
                        Ok(json) => json,
                        Err(err) => {
                            warn!("sse: serialize error: {err}");
                            continue;
                        }
                    };

                    return Some((Ok(Event::default().data(json)), rx));
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    warn!("sse: client lagged by {count} events");
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    let stream = sse_initial_stream().chain(event_stream);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(20))
            .text("keep-alive"),
    )
}

fn sse_initial_stream() -> impl Stream<Item = Result<Event, Infallible>> {
    stream::once(async { Ok(Event::default().comment(SSE_CONNECTED_COMMENT)) })
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn sse_initial_stream_sends_connected_comment_immediately() {
        let stream = sse_initial_stream();
        futures_util::pin_mut!(stream);
        let event = stream
            .next()
            .await
            .expect("initial sse event should be emitted immediately")
            .expect("initial sse event should serialize");

        drop(event);
    }

    #[test]
    fn position_update_serializes_full_position_payloads() {
        let event = RealtimeEvent::PositionUpdate {
            trader_id: "trader_1".to_string(),
            positions: vec![sample_position_payload()],
        };

        let value = serde_json::to_value(event).expect("serialize position update event");

        assert_eq!(
            value,
            json!({
                "type": "position_update",
                "trader_id": "trader_1",
                "positions": [
                    {
                        "id": "position_1",
                        "trader_id": "trader_1",
                        "symbol": "BTCUSDT",
                        "side": "LONG",
                        "quantity": 1.25,
                        "entry_price": 256.5,
                        "mark_price": 260.75,
                        "liquidation_price": 128.0,
                        "leverage": 5,
                        "margin_mode": "cross",
                        "unrealized_pnl": 12.5,
                        "realized_pnl": -2.5,
                        "status": "open",
                        "opened_at": 1_700_000_000,
                        "closed_at": null,
                        "updated_at": 1_700_000_900
                    }
                ]
            })
        );
    }

    #[test]
    fn position_update_deserializes_typed_position_payloads() {
        let event: RealtimeEvent = serde_json::from_value(json!({
            "type": "position_update",
            "trader_id": "trader_1",
            "positions": [
                {
                    "id": "position_1",
                    "trader_id": "trader_1",
                    "symbol": "BTCUSDT",
                    "side": "LONG",
                    "quantity": 1.25,
                    "entry_price": 256.5,
                    "mark_price": 260.75,
                    "liquidation_price": 128.0,
                    "leverage": 5,
                    "margin_mode": "cross",
                    "unrealized_pnl": 12.5,
                    "realized_pnl": -2.5,
                    "status": "open",
                    "opened_at": 1_700_000_000,
                    "closed_at": null,
                    "updated_at": 1_700_000_900
                }
            ]
        }))
        .expect("deserialize position update event");

        let RealtimeEvent::PositionUpdate {
            trader_id,
            positions,
        } = event
        else {
            panic!("expected position update event");
        };

        assert_eq!(trader_id, "trader_1");
        assert_eq!(positions.len(), 1);

        let position = &positions[0];
        assert_eq!(position.id, "position_1");
        assert_eq!(position.trader_id, "trader_1");
        assert_eq!(position.symbol, "BTCUSDT");
        assert_eq!(position.side, "LONG");
        assert_eq!(position.quantity, 1.25);
        assert_eq!(position.entry_price, 256.5);
        assert_eq!(position.mark_price, 260.75);
        assert_eq!(position.liquidation_price, 128.0);
        assert_eq!(position.leverage, 5);
        assert_eq!(position.margin_mode, "cross");
        assert_eq!(position.unrealized_pnl, 12.5);
        assert_eq!(position.realized_pnl, -2.5);
        assert_eq!(position.status, "open");
        assert_eq!(position.opened_at, 1_700_000_000);
        assert_eq!(position.closed_at, None);
        assert_eq!(position.updated_at, 1_700_000_900);
    }

    fn sample_position_payload() -> PositionPayload {
        PositionPayload {
            id: "position_1".to_string(),
            trader_id: "trader_1".to_string(),
            symbol: "BTCUSDT".to_string(),
            side: "LONG".to_string(),
            quantity: 1.25,
            entry_price: 256.5,
            mark_price: 260.75,
            liquidation_price: 128.0,
            leverage: 5,
            margin_mode: "cross".to_string(),
            unrealized_pnl: 12.5,
            realized_pnl: -2.5,
            status: "open".to_string(),
            opened_at: 1_700_000_000,
            closed_at: None,
            updated_at: 1_700_000_900,
        }
    }
}
