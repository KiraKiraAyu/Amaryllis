//! Shared runtime event constants and canonical catalog.
//!
//! Keep all runtime event type strings centralized here to avoid drift
//! between emitters (`engine`) and readers/aggregators (`routes`).

#[derive(Debug, Clone, Copy)]
pub struct RuntimeEventCatalogItem {
    pub event_type: &'static str,
    pub description: &'static str,
}

pub const EVENT_LIVE_RISK_SNAPSHOT: &str = "live_risk_snapshot";
pub const EVENT_LIVE_ORDER_SUBMITTED: &str = "live_order_submitted";
pub const EVENT_LIVE_OPEN_SKIPPED_CONSTRAINTS: &str = "live_open_skipped_constraints";
pub const EVENT_LIVE_OPEN_SKIPPED_MEDIUM_RISK: &str = "live_open_skipped_medium_risk";
pub const EVENT_LIVE_OPEN_USED_MARKET_FALLBACK: &str = "live_open_used_market_fallback";

pub const EVENT_CANCEL_REPLACE_THROTTLED: &str = "cancel_replace_throttled";
pub const EVENT_CANCEL_REPLACE_SUCCEEDED: &str = "cancel_replace_succeeded";
pub const EVENT_CANCEL_REPLACE_USED_MARKET_FALLBACK: &str = "cancel_replace_used_market_fallback";

pub const EVENT_STALE_INTENT_RECONCILE_TERMINAL: &str = "stale_intent_reconcile_terminal";
pub const EVENT_STALE_INTENT_RECONCILE_PENDING: &str = "stale_intent_reconcile_pending";

pub const EVENT_BUDGET_CIRCUIT_BREAKER: &str = "budget_circuit_breaker";

pub const CANONICAL_RUNTIME_EVENT_TYPES: &[RuntimeEventCatalogItem] = &[
    RuntimeEventCatalogItem {
        event_type: EVENT_LIVE_RISK_SNAPSHOT,
        description: "Risk decision snapshot for a live execution cycle.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_LIVE_ORDER_SUBMITTED,
        description: "Live order submitted to exchange (open/close/replace).",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_LIVE_OPEN_SKIPPED_CONSTRAINTS,
        description: "Open decision skipped because the calculated order missed exchange minimum constraints.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_LIVE_OPEN_SKIPPED_MEDIUM_RISK,
        description: "Open decision skipped due to medium risk guard.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_LIVE_OPEN_USED_MARKET_FALLBACK,
        description: "Open path used market fallback after limit-first attempt.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_CANCEL_REPLACE_THROTTLED,
        description: "Cancel-replace skipped due to throttling window/attempt limits.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_CANCEL_REPLACE_SUCCEEDED,
        description: "Cancel-replace flow successfully placed a replacement order.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_CANCEL_REPLACE_USED_MARKET_FALLBACK,
        description: "Cancel-replace flow fell back to market order.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_STALE_INTENT_RECONCILE_TERMINAL,
        description: "Stale submitted intent reconciled to a terminal status.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_STALE_INTENT_RECONCILE_PENDING,
        description: "Stale submitted intent observed non-terminal and touched.",
    },
    RuntimeEventCatalogItem {
        event_type: EVENT_BUDGET_CIRCUIT_BREAKER,
        description: "Budget circuit breaker triggered: unrealized loss exceeded the budget limit.",
    },
];

#[inline]
pub fn canonical_runtime_event_types() -> &'static [RuntimeEventCatalogItem] {
    CANONICAL_RUNTIME_EVENT_TYPES
}
