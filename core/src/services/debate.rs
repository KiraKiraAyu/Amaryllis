use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    contracts::debates::{
        DebateActionPayload, DebateDetailPayload, DebateExecutionPayload, DebateListPayload,
        DebateMessagePayload, DebateMessagesPayload, DebatePersonalitiesPayload,
        DebatePersonalityPayload, DebateVotesPayload,
    },
    error::{AppError, Result as AppResult},
    realtime::RealtimeHub,
    repositories::{
        debates::{CreateDebateMessageRecord, CreateDebateRecord, DebateRepo},
        models::ResolvedModelRecord,
    },
    services::llm::{LlmMessage, LlmService},
};

// ===== Personalities =====

/// Predefined AI personalities for the Debate Arena.
const PERSONALITIES: &[(&str, &str)] = &[
    (
        "bull",
        "Optimistic and growth-oriented. Favors long positions and tends to see upside potential.",
    ),
    (
        "bear",
        "Pessimistic and risk-averse. Favors short positions and focuses on downside risks.",
    ),
    (
        "neutral",
        "Balanced and data-driven. Takes positions only on clear technical signals.",
    ),
    (
        "contrarian",
        "Goes against the prevailing trend. Looks for overextended moves and reversals.",
    ),
    (
        "risk_manager",
        "Conservative. Prioritizes capital preservation and tight stop losses.",
    ),
    (
        "quant",
        "Statistical mindset. Relies on quantitative indicators and probability distributions.",
    ),
    (
        "fundamentals",
        "Macro-aware. Considers on-chain, funding rates, and market structure.",
    ),
];

// ===== Status =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum DebateStatus {
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl std::fmt::Display for DebateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DebateStatus::Pending => "pending",
            DebateStatus::Running => "running",
            DebateStatus::Completed => "completed",
            DebateStatus::Cancelled => "cancelled",
            DebateStatus::Failed => "failed",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct DebateService {
    debate_repo: Arc<DebateRepo>,
    realtime_hub: RealtimeHub,
    llm_service: Arc<LlmService>,
}

impl DebateService {
    pub fn new(
        debate_repo: Arc<DebateRepo>,
        realtime_hub: RealtimeHub,
        llm_service: Arc<LlmService>,
    ) -> Self {
        Self {
            debate_repo,
            realtime_hub,
            llm_service,
        }
    }

    pub async fn list(&self) -> DebateListPayload {
        let debates = self.debate_repo.list(100).await.unwrap_or_default();
        let count = debates.len();
        DebateListPayload { debates, count }
    }

    pub async fn create(
        &self,
        name: Option<String>,
        symbol: Option<String>,
        max_rounds: Option<i64>,
        participants: Option<Vec<String>>,
    ) -> AppResult<DebateActionPayload> {
        let create_request = DebateCreateRequest {
            name: name.unwrap_or_else(|| "Debate".to_string()),
            symbol: symbol.unwrap_or_else(|| "BTCUSDT".to_string()),
            max_rounds,
            participants,
        };

        let id = create_debate(&self.debate_repo, &create_request)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to create debate: {err}")))?;

        Ok(DebateActionPayload {
            id,
            message: "Debate created",
        })
    }

    pub fn personalities(&self) -> DebatePersonalitiesPayload {
        let personalities = PERSONALITIES
            .iter()
            .map(|(id, description)| DebatePersonalityPayload {
                id,
                name: id,
                description,
            })
            .collect();
        DebatePersonalitiesPayload { personalities }
    }

    pub async fn get(&self, debate_id: &str) -> AppResult<DebateDetailPayload> {
        let debate = self
            .debate_repo
            .get(debate_id)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to get debate: {err}")))?
            .ok_or_else(|| AppError::NotFound("Debate not found".into()))?;
        Ok(DebateDetailPayload { debate })
    }

    pub async fn delete(&self, debate_id: &str) -> AppResult<DebateMessagePayload> {
        delete_debate(&self.debate_repo, debate_id)
            .await
            .map_err(|err| AppError::BadRequest(err.into()))?;

        Ok(DebateMessagePayload {
            message: "Debate deleted",
        })
    }

    pub async fn start(&self, debate_id: &str) -> AppResult<DebateActionPayload> {
        let available = self.llm_service.list_runnable_for_user().await?;

        start_debate(
            self.debate_repo.clone(),
            debate_id.to_string(),
            available,
            self.llm_service.clone(),
            self.realtime_hub.clone(),
        )
        .await
        .map_err(|err| AppError::BadRequest(err.into()))?;

        Ok(DebateActionPayload {
            id: debate_id.to_string(),
            message: "Debate started",
        })
    }

    pub fn cancel(&self, debate_id: &str) -> AppResult<DebateActionPayload> {
        cancel_debate(debate_id).map_err(|err| AppError::BadRequest(err.into()))?;

        Ok(DebateActionPayload {
            id: debate_id.to_string(),
            message: "Debate cancelled",
        })
    }

    pub async fn execution(&self, debate_id: &str) -> AppResult<DebateExecutionPayload> {
        let debate = self
            .debate_repo
            .get(debate_id)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to get debate: {err}")))?
            .ok_or_else(|| AppError::NotFound("Debate not found".into()))?;

        Ok(DebateExecutionPayload {
            id: debate_id.to_string(),
            final_decision: debate["final_decision"]
                .as_str()
                .unwrap_or("HOLD")
                .to_string(),
            final_reasoning: debate["final_reasoning"].as_str().unwrap_or("").to_string(),
            message: "Debate result retrieved",
        })
    }

    pub async fn messages(&self, debate_id: &str) -> DebateMessagesPayload {
        let messages = self
            .debate_repo
            .list_messages(debate_id)
            .await
            .unwrap_or_default();
        let count = messages.len();
        DebateMessagesPayload { messages, count }
    }

    pub async fn votes(&self, debate_id: &str) -> DebateVotesPayload {
        let votes = self
            .debate_repo
            .votes(debate_id)
            .await
            .unwrap_or_else(|_| json!([]));
        DebateVotesPayload { votes }
    }
}

// ===== Time helpers =====

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ===== Runner =====

/// Configuration for creating a debate.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DebateCreateRequest {
    pub name: String,
    pub symbol: String,
    pub max_rounds: Option<i64>,
    pub participants: Option<Vec<String>>, // personality names
}

/// Create a new debate row in DB without starting it.
async fn create_debate(
    debate_repo: &DebateRepo,
    req: &DebateCreateRequest,
) -> Result<String, crate::database::DbErr> {
    let id = Uuid::now_v7().to_string();
    let now = now_ts();
    let max_rounds = req.max_rounds.unwrap_or(3).clamp(1, 10);
    let participants = req.participants.clone().unwrap_or_else(|| {
        vec![
            "bull".to_string(),
            "bear".to_string(),
            "neutral".to_string(),
        ]
    });
    let participants_json = serde_json::to_string(&participants).unwrap_or_default();

    debate_repo
        .create(CreateDebateRecord {
            id: id.clone(),
            name: req.name.trim().to_string(),
            symbol: req.symbol.trim().to_uppercase(),
            status: DebateStatus::Pending.to_string(),
            max_rounds,
            participants_json,
            created_at: now,
            updated_at: now,
        })
        .await?;
    Ok(id)
}

/// Delete a debate (only if pending or completed/failed).
async fn delete_debate(debate_repo: &DebateRepo, debate_id: &str) -> Result<(), String> {
    let mgr = get_debate_manager();
    let is_running = mgr
        .lock()
        .ok()
        .map(|g| g.contains_key(debate_id))
        .unwrap_or(false);
    if is_running {
        return Err("Debate is currently running; cancel before deleting".into());
    }

    let rows_affected = debate_repo
        .delete(debate_id)
        .await
        .map_err(|e| e.to_string())?;
    if rows_affected == 0 {
        return Err("Debate not found".into());
    }
    Ok(())
}

/// Cancel a running debate by signalling the background task.
fn cancel_debate(debate_id: &str) -> Result<(), String> {
    let mgr = get_debate_manager();
    let mut guard = mgr.lock().unwrap();
    if let Some(tx) = guard.remove(debate_id) {
        let _ = tx.send(());
        Ok(())
    } else {
        Err("Debate not running".into())
    }
}

/// Global manager: debate_id -> cancel_tx
type DebateManagerInner = HashMap<String, tokio::sync::oneshot::Sender<()>>;
type SharedDebateManager = Arc<Mutex<DebateManagerInner>>;

static DEBATE_MANAGER: OnceLock<SharedDebateManager> = OnceLock::new();

fn get_debate_manager() -> SharedDebateManager {
    DEBATE_MANAGER
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

/// Start the debate: spawn background task that runs all rounds sequentially.
async fn start_debate(
    debate_repo: Arc<DebateRepo>,
    debate_id: String,
    ai_configs: Vec<ResolvedModelRecord>,
    llm_service: Arc<LlmService>,
    realtime_hub: RealtimeHub,
) -> Result<(), String> {
    // Fetch debate config
    let debate = debate_repo
        .get(&debate_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Debate not found")?;
    let status = debate["status"].as_str().unwrap_or("");
    if status != "pending" && status != "failed" {
        return Err(format!("Debate is {}; cannot start", status));
    }

    debate_repo
        .update_status(&debate_id, &DebateStatus::Running.to_string(), now_ts())
        .await
        .map_err(|e| e.to_string())?;

    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mgr = get_debate_manager();
        mgr.lock().unwrap().insert(debate_id.clone(), cancel_tx);
    }

    let max_rounds = debate["max_rounds"].as_i64().unwrap_or(3) as usize;
    let symbol = debate["symbol"].as_str().unwrap_or("BTCUSDT").to_string();
    let participants: Vec<String> =
        serde_json::from_value(debate["participants"].clone()).unwrap_or_default();

    tokio::spawn(run_debate_task(
        debate_repo,
        debate_id,
        participants,
        ai_configs,
        llm_service,
        max_rounds,
        symbol,
        cancel_rx,
        realtime_hub,
    ));

    Ok(())
}

async fn run_debate_task(
    debate_repo: Arc<DebateRepo>,
    debate_id: String,
    participants: Vec<String>,
    ai_configs: Vec<ResolvedModelRecord>,
    llm_service: Arc<LlmService>,
    max_rounds: usize,
    symbol: String,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
    realtime_hub: RealtimeHub,
) {
    // Build LLM clients keyed by personality (or cycle through configs)
    let mut conversation_history: Vec<(String, String)> = Vec::new(); // (personality, content)

    for round in 1..=max_rounds {
        if cancel_rx.try_recv().is_ok() {
            let _ = debate_repo
                .update_status(&debate_id, &DebateStatus::Cancelled.to_string(), now_ts())
                .await;
            remove_from_manager(&debate_id);
            return;
        }

        // Each participant takes a turn per round
        for (i, personality) in participants.iter().enumerate() {
            let personality_desc = PERSONALITIES
                .iter()
                .find(|(p, _)| *p == personality.as_str())
                .map(|(_, d)| *d)
                .unwrap_or("impartial observer");

            // Build prompt including conversation so far
            let history_text = conversation_history
                .iter()
                .map(|(p, c)| format!("[{}]: {}", p, c))
                .collect::<Vec<_>>()
                .join("\n\n");

            let prompt = format!(
                r#"## Debate Round {round} — You are the "{personality}" analyst

**Symbol**: {symbol}
**Your perspective**: {personality_desc}

**Previous arguments**:
{history}

Now give your analysis of {symbol}. End your response with a vote: LONG, SHORT, or HOLD.
Format: Your analysis paragraph... then on a new line: VOTE: LONG|SHORT|HOLD"#,
                round = round,
                personality = personality,
                symbol = symbol,
                personality_desc = personality_desc,
                history = if history_text.is_empty() {
                    "(none yet)".to_string()
                } else {
                    history_text.clone()
                },
            );

            // Pick AI config for this participant (cycle through provided configs)
            let cfg_idx = i % ai_configs.len().max(1);
            let response = if let Some(model) = ai_configs.get(cfg_idx) {
                let messages = vec![LlmMessage {
                    role: "user".to_string(),
                    content: prompt.clone(),
                }];
                let system_prompt = format!(
                    "You are the '{}' trading analyst. {}",
                    personality, personality_desc
                );
                llm_service
                    .chat_with_model(model, messages, Some(&system_prompt))
                    .await
                    .unwrap_or_else(|e| format!("[LLM error: {e}]"))
            } else {
                // No AI configured — use a simple simulated response
                format!(
                    "As a {} analyst, I see mixed signals for {}. VOTE: HOLD",
                    personality, symbol
                )
            };

            // Extract vote from response
            let vote = extract_vote_from_response(&response);

            // Persist message
            let msg_id = Uuid::now_v7().to_string();
            let _ = debate_repo
                .insert_message(CreateDebateMessageRecord {
                    id: msg_id,
                    debate_id: debate_id.clone(),
                    round: round as i64,
                    personality: personality.clone(),
                    role: "assistant".to_string(),
                    content: response.clone(),
                    vote: vote.clone(),
                    created_at: now_ts(),
                })
                .await;

            conversation_history.push((personality.clone(), response.chars().take(500).collect()));

            // Push debate message to realtime clients in real-time
            realtime_hub.publish(crate::realtime::RealtimeEvent::DebateMessage {
                debate_id: debate_id.clone(),
                round: round as i64,
                personality: personality.clone(),
                content: response.chars().take(500).collect(),
                vote: vote.clone(),
            });

            // Update current round
            let _ = debate_repo
                .update_current_round(&debate_id, round as i64, now_ts())
                .await;

            // Yield between turns
            tokio::task::yield_now().await;
        }
    }

    // Compute final decision from all votes
    let tally = debate_repo.vote_tally(&debate_id).await.unwrap_or_default();

    let final_decision = tally
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(v, _)| v.clone())
        .unwrap_or_else(|| "HOLD".to_string());

    let final_reasoning = format!(
        "After {} rounds with {} participants, the majority vote was {} (tally: {:?})",
        max_rounds,
        participants.len(),
        final_decision,
        tally
    );

    let _ = debate_repo
        .complete(
            &debate_id,
            &DebateStatus::Completed.to_string(),
            final_decision.clone(),
            final_reasoning.clone(),
            now_ts(),
        )
        .await;

    // Push debate finished event to realtime clients
    realtime_hub.publish(crate::realtime::RealtimeEvent::DebateFinished {
        debate_id: debate_id.clone(),
        status: "completed".to_string(),
        final_decision,
        final_reasoning,
    });

    remove_from_manager(&debate_id);
}

fn extract_vote_from_response(response: &str) -> String {
    let upper = response.to_uppercase();
    // Look for "VOTE: XXX" pattern first
    if let Some(pos) = upper.find("VOTE:") {
        let rest = &upper[pos + 5..]
            .trim_start()
            .chars()
            .take(10)
            .collect::<String>();
        if rest.contains("LONG") {
            return "LONG".to_string();
        }
        if rest.contains("SHORT") {
            return "SHORT".to_string();
        }
        if rest.contains("HOLD") {
            return "HOLD".to_string();
        }
    }
    // Fallback: count occurrences
    let longs = upper.matches("LONG").count();
    let shorts = upper.matches("SHORT").count();
    let holds = upper.matches("HOLD").count();
    if longs > shorts && longs > holds {
        "LONG".to_string()
    } else if shorts > longs && shorts > holds {
        "SHORT".to_string()
    } else {
        "HOLD".to_string()
    }
}

fn remove_from_manager(debate_id: &str) {
    let mgr = get_debate_manager();
    let mut g = mgr.lock().unwrap_or_else(|e| e.into_inner());
    g.remove(debate_id);
}
