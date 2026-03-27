use chrono::Utc;
use uuid::Uuid;

use crate::domain::game::GameResult;
use crate::domain::settlement::{Settlement, SettlementStatus};
use crate::errors::AppError;
use crate::state::AppState;

pub async fn initiate(result: &GameResult, state: &AppState) -> Result<(), AppError> {
    let now = Utc::now();
    let settlement = Settlement {
        settlement_id: Uuid::new_v4(),
        game_id: result.game_id,
        status: SettlementStatus::Pending,
        tx_hash: None,
        block_number: None,
        confirmed_at: None,
        retry_count: 0,
        error_message: None,
        created_at: now,
        updated_at: now,
    };

    // For in-memory we store it via the game_store (no dedicated settlement store in memory)
    // The settlement is tracked in-memory here
    // TODO: In a real system, this would use a SettlementStore port

    match state
        .settlement
        .settle(
            result.game_id,
            &result.winners,
            &result.rake_atomic,
            &result.signed_result_hash,
        )
        .await
    {
        Ok(tx_hash) => {
            // Poll for confirmation with exponential backoff
            let delays = [1u64, 2, 4, 8, 16];
            let mut confirmed = false;
            let mut block_number = None;

            for delay in &delays {
                tokio::time::sleep(tokio::time::Duration::from_secs(*delay)).await;
                match state.settlement.check_confirmation(&tx_hash).await {
                    Ok(Some(bn)) => {
                        confirmed = true;
                        block_number = Some(bn);
                        break;
                    }
                    _ => {}
                }
            }

            if confirmed {
                state
                    .event_bus
                    .publish(
                        "SETTLEMENT_COMPLETED",
                        &serde_json::json!({
                            "game_id": result.game_id.to_string(),
                            "tx_hash": tx_hash,
                            "block_number": block_number,
                        }),
                    )
                    .await?;
            } else {
                state
                    .event_bus
                    .publish(
                        "SETTLEMENT_FAILED",
                        &serde_json::json!({
                            "game_id": result.game_id.to_string(),
                            "tx_hash": tx_hash,
                            "error": "Confirmation timeout",
                        }),
                    )
                    .await?;
            }
        }
        Err(e) => {
            state
                .event_bus
                .publish(
                    "SETTLEMENT_FAILED",
                    &serde_json::json!({
                        "game_id": result.game_id.to_string(),
                        "error": e.to_string(),
                    }),
                )
                .await?;
        }
    }

    Ok(())
}

pub async fn get_settlement(game_id: Uuid, _state: &AppState) -> Result<Settlement, AppError> {
    // TODO: Needs a SettlementStore port. For now return not found.
    // The full implementation would look up from a settlement store.
    Err(AppError::NotFound)
}

pub async fn get_agent_history(
    _agent_id: Uuid,
    _limit: i64,
    _offset: i64,
    _state: &AppState,
) -> Result<Vec<Settlement>, AppError> {
    Ok(vec![])
}
