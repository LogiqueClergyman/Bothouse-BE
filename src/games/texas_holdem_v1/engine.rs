use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::game::{Game, GamePlayer, GameResult, LoserEntry, WinnerEntry};
use crate::domain::DomainError as DE;

use super::deck::shuffle_deck;
use super::hand_evaluator::evaluate_best_hand;

pub struct TexasHoldemV1;

impl Game for TexasHoldemV1 {
    fn game_type(&self) -> &'static str {
        "texas_holdem_v1"
    }

    fn display_name(&self) -> &'static str {
        "Texas Hold'em Poker"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn min_players(&self) -> usize {
        2
    }

    fn max_players(&self) -> usize {
        9
    }

    fn turn_timeout_ms(&self) -> u64 {
        60000
    }

    fn init(&self, players: Vec<GamePlayer>, seed: [u8; 32]) -> Result<Value, DE> {
        let buy_in: u128 = players
            .first()
            .and_then(|p| p.stack_atomic.parse().ok())
            .unwrap_or(0);

        let small_blind_atomic = buy_in / 100;
        let big_blind_atomic = buy_in / 50;

        let mut deck = shuffle_deck(seed);

        let dealer_seat = 1i16;
        let n = players.len() as i16;
        let small_blind_seat = if n == 2 { dealer_seat } else { (dealer_seat % n) + 1 };
        let big_blind_seat = (small_blind_seat % n) + 1;
        let first_to_act = (big_blind_seat % n) + 1;

        let mut player_states: Vec<Value> = Vec::new();

        for p in &players {
            let hole1 = deck.remove(0);
            let hole2 = deck.remove(0);

            let position = compute_position(p.seat_number, dealer_seat, n);

            let stack: u128 = p.stack_atomic.parse().unwrap_or(0);
            let (stack_after_blind, bet_this_round) = if p.seat_number == small_blind_seat {
                let sb = small_blind_atomic.min(stack);
                (stack - sb, sb)
            } else if p.seat_number == big_blind_seat {
                let bb = big_blind_atomic.min(stack);
                (stack - bb, bb)
            } else {
                (stack, 0)
            };

            player_states.push(json!({
                "agent_id": p.agent_id.to_string(),
                "seat_number": p.seat_number,
                "hole_cards": [hole1, hole2],
                "stack_atomic": stack_after_blind.to_string(),
                "status": "active",
                "position": position,
                "bet_this_round_atomic": bet_this_round.to_string(),
                "consecutive_timeouts": 0,
            }));
        }

        let pot = small_blind_atomic + big_blind_atomic;

        Ok(json!({
            "phase": "pre_flop",
            "deck": deck,
            "community_cards": [],
            "players": player_states,
            "pot_atomic": pot.to_string(),
            "side_pots": [],
            "current_bet_atomic": big_blind_atomic.to_string(),
            "dealer_seat": dealer_seat,
            "small_blind_seat": small_blind_seat,
            "big_blind_seat": big_blind_seat,
            "action_on_seat": first_to_act,
            "turn_number": 1,
            "small_blind_atomic": small_blind_atomic.to_string(),
            "big_blind_atomic": big_blind_atomic.to_string(),
            "buy_in_atomic": buy_in.to_string(),
            "last_aggressor_seat": big_blind_seat,
        }))
    }

    fn visible_state(&self, state: &Value, agent_id: Uuid) -> Result<Value, DE> {
        let mut visible = state.clone();
        let phase = state["phase"].as_str().unwrap_or("");
        let is_showdown = phase == "showdown" || phase == "completed";

        if let Some(players) = visible["players"].as_array_mut() {
            for player in players.iter_mut() {
                let pid = player["agent_id"].as_str().unwrap_or("");
                let is_owner = pid == agent_id.to_string();
                let player_status = player["status"].as_str().unwrap_or("active");
                let is_active = player_status == "active" || player_status == "all_in";

                if !is_owner && !(is_showdown && is_active) {
                    player["hole_cards"] = json!(null);
                }
            }
        }

        // Always remove the deck from visible state
        visible["deck"] = json!([]);

        Ok(visible)
    }

    fn valid_actions(&self, state: &Value, agent_id: Uuid) -> Result<Vec<String>, DE> {
        let phase = state["phase"].as_str().unwrap_or("");
        if phase == "showdown" || phase == "completed" || phase == "waiting" {
            return Ok(vec![]);
        }

        let action_on_seat = state["action_on_seat"].as_i64().unwrap_or(-1);

        // Find player
        let players = state["players"].as_array().ok_or_else(|| DE::StateParseError("no players".into()))?;
        let player = players
            .iter()
            .find(|p| p["agent_id"].as_str() == Some(&agent_id.to_string()));

        let player = match player {
            Some(p) => p,
            None => return Ok(vec![]),
        };

        let seat_number = player["seat_number"].as_i64().unwrap_or(-1);
        if seat_number != action_on_seat {
            return Ok(vec![]);
        }

        let status = player["status"].as_str().unwrap_or("active");
        if status != "active" {
            return Ok(vec![]);
        }

        let stack: u128 = player["stack_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
        let bet_this_round: u128 = player["bet_this_round_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
        let current_bet: u128 = state["current_bet_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
        let big_blind: u128 = state["big_blind_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
        let big_blind_seat = state["big_blind_seat"].as_i64().unwrap_or(-1);

        let to_call = current_bet.saturating_sub(bet_this_round);

        if to_call == 0 {
            // No bet this round or BB with option
            if stack == 0 {
                return Ok(vec![]);
            }
            return Ok(vec!["check".to_string(), "bet".to_string(), "fold".to_string()]);
        }

        // Facing a bet
        if stack <= to_call {
            return Ok(vec!["fold".to_string(), "all_in".to_string()]);
        }

        Ok(vec!["fold".to_string(), "call".to_string(), "raise".to_string()])
    }

    fn apply_action(
        &self,
        mut state: Value,
        agent_id: Uuid,
        action: &str,
        amount_atomic: Option<&str>,
    ) -> Result<Value, DE> {
        let agent_str = agent_id.to_string();
        let player_idx = {
            let players = state["players"].as_array().ok_or_else(|| DE::StateParseError("no players".into()))?;
            players
                .iter()
                .position(|p| p["agent_id"].as_str() == Some(&agent_str))
                .ok_or(DE::InvalidAction("Player not found".into()))?
        };

        let current_bet: u128 = state["current_bet_atomic"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        let big_blind: u128 = state["big_blind_atomic"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);

        let player_stack: u128 = state["players"][player_idx]["stack_atomic"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        let bet_this_round: u128 = state["players"][player_idx]["bet_this_round_atomic"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        let to_call = current_bet.saturating_sub(bet_this_round);

        match action {
            "fold" => {
                state["players"][player_idx]["status"] = json!("folded");
            }
            "check" => {
                if to_call > 0 {
                    return Err(DE::InvalidAction("Cannot check when facing a bet".into()));
                }
            }
            "call" => {
                let call_amount = to_call.min(player_stack);
                let new_stack = player_stack - call_amount;
                let new_bet = bet_this_round + call_amount;
                state["players"][player_idx]["stack_atomic"] = json!(new_stack.to_string());
                state["players"][player_idx]["bet_this_round_atomic"] = json!(new_bet.to_string());

                let pot: u128 = state["pot_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
                state["pot_atomic"] = json!((pot + call_amount).to_string());

                if new_stack == 0 {
                    state["players"][player_idx]["status"] = json!("all_in");
                }
            }
            "bet" => {
                let amount: u128 = amount_atomic
                    .ok_or_else(|| DE::InvalidAmount("amount required for bet".into()))?
                    .parse()
                    .map_err(|_| DE::InvalidAmount("invalid amount".into()))?;
                if amount < big_blind {
                    return Err(DE::InvalidAmount(format!("Bet must be >= big blind {}", big_blind)));
                }
                if amount > player_stack {
                    return Err(DE::InvalidAmount("Bet exceeds stack".into()));
                }

                let new_stack = player_stack - amount;
                let new_bet = bet_this_round + amount;
                state["players"][player_idx]["stack_atomic"] = json!(new_stack.to_string());
                state["players"][player_idx]["bet_this_round_atomic"] = json!(new_bet.to_string());
                state["current_bet_atomic"] = json!(new_bet.to_string());

                let pot: u128 = state["pot_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
                state["pot_atomic"] = json!((pot + amount).to_string());

                let seat = state["players"][player_idx]["seat_number"].as_i64().unwrap_or(0);
                state["last_aggressor_seat"] = json!(seat);

                if new_stack == 0 {
                    state["players"][player_idx]["status"] = json!("all_in");
                }
            }
            "raise" => {
                let amount: u128 = amount_atomic
                    .ok_or_else(|| DE::InvalidAmount("amount required for raise".into()))?
                    .parse()
                    .map_err(|_| DE::InvalidAmount("invalid amount".into()))?;
                if amount > player_stack {
                    return Err(DE::InvalidAmount("Raise exceeds stack".into()));
                }
                if amount < current_bet {
                    return Err(DE::InvalidAmount("Raise must be at least current bet".into()));
                }

                let new_stack = player_stack - amount;
                let new_bet = bet_this_round + amount;
                state["players"][player_idx]["stack_atomic"] = json!(new_stack.to_string());
                state["players"][player_idx]["bet_this_round_atomic"] = json!(new_bet.to_string());
                state["current_bet_atomic"] = json!(new_bet.to_string());

                let pot: u128 = state["pot_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
                state["pot_atomic"] = json!((pot + amount).to_string());

                let seat = state["players"][player_idx]["seat_number"].as_i64().unwrap_or(0);
                state["last_aggressor_seat"] = json!(seat);

                if new_stack == 0 {
                    state["players"][player_idx]["status"] = json!("all_in");
                }
            }
            "all_in" => {
                let new_bet = bet_this_round + player_stack;
                if new_bet > current_bet {
                    state["current_bet_atomic"] = json!(new_bet.to_string());
                    let seat = state["players"][player_idx]["seat_number"].as_i64().unwrap_or(0);
                    state["last_aggressor_seat"] = json!(seat);
                }
                state["players"][player_idx]["bet_this_round_atomic"] = json!(new_bet.to_string());

                let pot: u128 = state["pot_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
                state["pot_atomic"] = json!((pot + player_stack).to_string());

                state["players"][player_idx]["stack_atomic"] = json!("0");
                state["players"][player_idx]["status"] = json!("all_in");
            }
            _ => {
                return Err(DE::InvalidAction(format!("Unknown action: {}", action)));
            }
        }

        // Increment turn number
        let turn = state["turn_number"].as_i64().unwrap_or(0);
        state["turn_number"] = json!(turn + 1);

        // Update consecutive_timeouts if action was a real play (reset to 0)
        if !action.starts_with("timeout:") {
            state["players"][player_idx]["consecutive_timeouts"] = json!(0);
        }

        // Advance action
        state = advance_action(state)?;

        Ok(state)
    }

    fn is_terminal(&self, state: &Value) -> bool {
        let phase = state["phase"].as_str().unwrap_or("");
        phase == "completed"
    }

    fn result(&self, state: &Value, game_id: Uuid, rake_bps: u16) -> Option<GameResult> {
        if state["phase"].as_str()? != "completed" {
            return None;
        }

        let players = state["players"].as_array()?;

        // The on-chain escrow holds buy_in * num_players (total deposited).
        // The off-chain pot_atomic only tracks bets/blinds accumulated during play
        // and does NOT match the on-chain total_pot.
        // We must settle the full escrow amount to satisfy the Move contract:
        //   assert!(sum(payouts) + rake == total_pot)
        let buy_in: u128 = state["buy_in_atomic"].as_str().unwrap_or("0").parse().ok()?;
        let num_players = players.len() as u128;
        let pot: u128 = buy_in * num_players;

        let community_cards: Vec<String> = state["community_cards"]
            .as_array()?
            .iter()
            .filter_map(|c| c.as_str().map(String::from))
            .collect();

        let rake = (pot * rake_bps as u128) / 10000;
        let distributable = pot.saturating_sub(rake);

        // Evaluate winners
        let active_players: Vec<&Value> = players
            .iter()
            .filter(|p| {
                let status = p["status"].as_str().unwrap_or("");
                status == "active" || status == "all_in"
            })
            .collect();

        if active_players.is_empty() {
            return None;
        }

        let winners: Vec<usize> = if active_players.len() == 1 {
            vec![0]
        } else {
            // Simple case: evaluate best hand for each active player
            let mut player_scores: Vec<(usize, u32)> = active_players
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    let hole = p["hole_cards"].as_array()?;
                    let hole_cards: Vec<String> = hole
                        .iter()
                        .filter_map(|c| c.as_str().map(String::from))
                        .collect();
                    if hole_cards.len() < 2 {
                        return None;
                    }
                    let mut all_cards = hole_cards;
                    all_cards.extend(community_cards.clone());
                    let eval = evaluate_best_hand(&all_cards);
                    Some((i, eval.score))
                })
                .collect();
    
            player_scores.sort_by(|a, b| b.1.cmp(&a.1));
            // Ensure we have at least one score
            if player_scores.is_empty() {
                return None;
            }
            let top_score = player_scores.first().unwrap().1;
            player_scores
                .iter()
                .filter(|(_, s)| *s == top_score)
                .map(|(i, _)| *i)
                .collect()
        };

        let per_winner = distributable / winners.len() as u128;
        let remainder = distributable - per_winner * winners.len() as u128;

        let buy_in_atomic = state["buy_in_atomic"].as_str().unwrap_or("0").to_string();

        let winner_entries: Vec<WinnerEntry> = winners
            .iter()
            .enumerate()
            .filter_map(|(idx, &wi)| {
                let p = active_players[wi];
                let agent_id_str = p["agent_id"].as_str()?;
                let agent_id = agent_id_str.parse().ok()?;
                let wallet = p.get("wallet_address")
                    .or_else(|| players.iter().find(|pp| pp["agent_id"].as_str() == Some(agent_id_str))?.get("wallet_address"))
                    .and_then(|w| w.as_str())
                    .unwrap_or("");
                let amount = if idx == 0 { per_winner + remainder } else { per_winner };
                Some(WinnerEntry {
                    agent_id,
                    wallet_address: wallet.to_string(),
                    amount_won_atomic: amount.to_string(),
                })
            })
            .collect();

        let loser_entries: Vec<LoserEntry> = players
            .iter()
            .filter(|p| {
                let pid = p["agent_id"].as_str().unwrap_or("");
                !winner_entries.iter().any(|w| w.agent_id.to_string() == pid)
            })
            .filter_map(|p| {
                let agent_id = p["agent_id"].as_str()?.parse().ok()?;
                let wallet = p["wallet_address"].as_str().unwrap_or("").to_string();
                // TODO: track exact losses per player
                let amount_lost: u128 = buy_in_atomic.parse().unwrap_or(0);
                Some(LoserEntry {
                    agent_id,
                    wallet_address: wallet,
                    amount_lost_atomic: amount_lost.to_string(),
                })
            })
            .collect();

        // Compute result hash: keccak256 of game_id + winners + amounts + rake
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(game_id.as_bytes());
        for w in &winner_entries {
            hasher.update(w.wallet_address.as_bytes());
            hasher.update(w.amount_won_atomic.as_bytes());
        }
        hasher.update(rake.to_string().as_bytes());
        let hash = hasher.finalize();
        let signed_result_hash = format!("0x{}", hex::encode(hash));

        Some(GameResult {
            game_id,
            winners: winner_entries,
            losers: loser_entries,
            rake_atomic: rake.to_string(),
            rake_rate_bps: rake_bps as i16,
            signed_result_hash,
        })
    }

    fn timeout_action(&self, _state: &Value, _agent_id: Uuid) -> String {
        "fold".to_string()
    }

    fn verify_action_signature(
        &self,
        game_id: Uuid,
        turn_number: i64,
        action: &str,
        amount_atomic: Option<&str>,
        signature: &str,
        wallet_address: &str,
        chain_type: &str,
    ) -> Result<bool, DE> {
        let mut message = Vec::new();
        message.extend_from_slice(game_id.as_bytes());
        message.extend_from_slice(&turn_number.to_be_bytes());
        message.extend_from_slice(action.as_bytes());
        message.extend_from_slice(amount_atomic.unwrap_or("").as_bytes());

        match chain_type {
            "evm" => verify_evm_signature(&message, signature, wallet_address),
            "onechain" => {
                // Ed25519: verify against known public key derived from wallet address.
                // wallet_address for OneChain is the 0x+64 hex Sui address (32-byte hash of pubkey).
                // Full verification requires the public key — here we use the settlement adapter helper.
                // For now delegate to EVM-style check; full Ed25519 path wired in auth_service.
                Err(DE::InvalidAction("OneChain signature verification requires public key lookup".into()))
            }
            _ => Err(DE::InvalidAction(format!("Unknown chain_type: {}", chain_type))),
        }
    }
}

fn verify_evm_signature(message: &[u8], signature: &str, wallet_address: &str) -> Result<bool, DE> {
    use sha3::{Digest, Keccak256};

    let msg_hash = Keccak256::digest(message);

    // EIP-191 personal sign
    let prefixed_msg = format!("\x19Ethereum Signed Message:\n{}", msg_hash.len());
    let mut prefixed = prefixed_msg.into_bytes();
    prefixed.extend_from_slice(&msg_hash);
    let final_hash = Keccak256::digest(&prefixed);

    let sig: alloy::primitives::Signature = signature.parse()
        .map_err(|_| DE::InvalidAction("Invalid signature format".into()))?;
    let mut h = [0u8; 32];
    h.copy_from_slice(&final_hash[..32]);
    let recovered = sig.recover_address_from_prehash(&alloy::primitives::B256::from(h))
        .map_err(|_| DE::InvalidAction("Signature recovery failed".into()))?;
    Ok(format!("{:#x}", recovered).to_lowercase() == wallet_address.to_lowercase())
}

/// Check if the betting round is complete and advance to next phase if so.
fn advance_action(mut state: Value) -> Result<Value, DE> {
    let phase = state["phase"].as_str().unwrap_or("").to_string();
    let players = state["players"].as_array().cloned().unwrap_or_default();

    let active_non_allin: Vec<&Value> = players
        .iter()
        .filter(|p| p["status"].as_str() == Some("active"))
        .collect();

    let active_all: Vec<&Value> = players
        .iter()
        .filter(|p| {
            let s = p["status"].as_str().unwrap_or("");
            s == "active" || s == "all_in"
        })
        .collect();

    // If only one player hasn't folded, they win
    if active_all.len() == 1 {
        state["phase"] = json!("completed");
        return Ok(state);
    }

    let current_bet: u128 = state["current_bet_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
    let last_aggressor_seat = state["last_aggressor_seat"].as_i64().unwrap_or(0);

    // Check if betting round is complete: all active (non-all-in) players have matched current bet
    let round_complete = active_non_allin.iter().all(|p| {
        let bet: u128 = p["bet_this_round_atomic"].as_str().unwrap_or("0").parse().unwrap_or(0);
        bet >= current_bet
    }) || active_non_allin.is_empty();

    if round_complete {
        // Advance to next phase
        state = next_phase(state)?;
    } else {
        // Find next active player
        let action_on_seat = state["action_on_seat"].as_i64().unwrap_or(1);
        let n = players.len() as i64;
        let mut next_seat = (action_on_seat % n) + 1;

        for _ in 0..n {
            let player = players.iter().find(|p| p["seat_number"].as_i64() == Some(next_seat));
            if let Some(p) = player {
                if p["status"].as_str() == Some("active") {
                    state["action_on_seat"] = json!(next_seat);
                    return Ok(state);
                }
            }
            next_seat = (next_seat % n) + 1;
        }
    }

    Ok(state)
}

fn next_phase(mut state: Value) -> Result<Value, DE> {
    let phase = state["phase"].as_str().unwrap_or("").to_string();
    let mut deck: Vec<String> = state["deck"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|c| c.as_str().map(String::from))
        .collect();
    let mut community: Vec<String> = state["community_cards"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|c| c.as_str().map(String::from))
        .collect();

    // Reset bets
    let players = state["players"].as_array_mut().unwrap();
    for p in players.iter_mut() {
        if p["status"].as_str() == Some("active") || p["status"].as_str() == Some("all_in") {
            p["bet_this_round_atomic"] = json!("0");
        }
    }
    state["current_bet_atomic"] = json!("0");

    let next_phase_str = match phase.as_str() {
        "pre_flop" => {
            // Burn 1, deal 3
            if !deck.is_empty() { deck.remove(0); }
            for _ in 0..3 {
                if !deck.is_empty() {
                    community.push(deck.remove(0));
                }
            }
            "flop"
        }
        "flop" => {
            if !deck.is_empty() { deck.remove(0); }
            if !deck.is_empty() {
                community.push(deck.remove(0));
            }
            "turn"
        }
        "turn" => {
            if !deck.is_empty() { deck.remove(0); }
            if !deck.is_empty() {
                community.push(deck.remove(0));
            }
            "river"
        }
        "river" => {
            "showdown"
        }
        "showdown" => {
            "completed"
        }
        _ => "completed",
    };

    state["deck"] = json!(deck);
    state["community_cards"] = json!(community);
    state["phase"] = json!(next_phase_str);

    if next_phase_str == "showdown" || next_phase_str == "completed" {
        state["phase"] = json!("completed");
        return Ok(state);
    }

    // Find first active player after dealer for new phase
    let dealer_seat = state["dealer_seat"].as_i64().unwrap_or(1);
    let players_arr = state["players"].as_array().cloned().unwrap_or_default();
    let n = players_arr.len() as i64;

    let mut next_seat = (dealer_seat % n) + 1;
    for _ in 0..n {
        let player = players_arr.iter().find(|p| p["seat_number"].as_i64() == Some(next_seat));
        if let Some(p) = player {
            if p["status"].as_str() == Some("active") {
                state["action_on_seat"] = json!(next_seat);
                return Ok(state);
            }
        }
        next_seat = (next_seat % n) + 1;
    }

    // All players all-in, run out remaining streets
    state["phase"] = json!("completed");
    Ok(state)
}

fn compute_position(seat: i16, dealer: i16, n: i16) -> &'static str {
    let offset = ((seat - dealer - 1).rem_euclid(n)) as usize;
    match n {
        2 => match offset {
            0 => "BTN",
            _ => "BB",
        },
        _ => match offset {
            0 => "BTN",
            1 => "SB",
            2 => "BB",
            _ => "MP",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::game::PlayerStatus;

    fn make_players(n: usize) -> Vec<GamePlayer> {
        (0..n)
            .map(|i| GamePlayer {
                game_id: Uuid::nil(),
                agent_id: Uuid::new_v4(),
                wallet_address: format!("0x{:040x}", i + 1),
                seat_number: (i + 1) as i16,
                stack_atomic: "1000000000000000000".to_string(), // 1 ETH
                status: PlayerStatus::Active,
                consecutive_timeouts: 0,
            })
            .collect()
    }

    #[test]
    fn test_init_creates_valid_state() {
        let game = TexasHoldemV1;
        let players = make_players(2);
        let seed = [0u8; 32];
        let state = game.init(players, seed).unwrap();

        assert_eq!(state["phase"], "pre_flop");
        assert_eq!(state["players"].as_array().unwrap().len(), 2);
        assert!(state["pot_atomic"].as_str().unwrap().parse::<u128>().unwrap() > 0);
    }

    #[test]
    fn test_valid_actions_on_turn() {
        let game = TexasHoldemV1;
        let players = make_players(2);
        let seed = [0u8; 32];
        let state = game.init(players.clone(), seed).unwrap();

        let action_seat = state["action_on_seat"].as_i64().unwrap();
        let acting_player = players
            .iter()
            .find(|p| p.seat_number == action_seat as i16)
            .unwrap();

        let actions = game.valid_actions(&state, acting_player.agent_id).unwrap();
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_fold_advances_phase() {
        let game = TexasHoldemV1;
        let players = make_players(2);
        let seed = [1u8; 32];
        let state = game.init(players.clone(), seed).unwrap();

        let action_seat = state["action_on_seat"].as_i64().unwrap();
        let acting = players
            .iter()
            .find(|p| p.seat_number == action_seat as i16)
            .unwrap();

        let new_state = game.apply_action(state, acting.agent_id, "fold", None).unwrap();
        assert_eq!(new_state["phase"], "completed");
    }

    #[test]
    fn test_timeout_action_is_fold() {
        let game = TexasHoldemV1;
        let state = json!({"phase": "pre_flop"});
        let action = game.timeout_action(&state, Uuid::new_v4());
        assert_eq!(action, "fold");
    }

    #[test]
    fn test_visible_state_hides_hole_cards() {
        let game = TexasHoldemV1;
        let players = make_players(2);
        let seed = [2u8; 32];
        let state = game.init(players.clone(), seed).unwrap();

        let visible = game.visible_state(&state, players[0].agent_id).unwrap();
        let ps = visible["players"].as_array().unwrap();

        // Player 0 should see their own cards
        let own = ps.iter().find(|p| p["agent_id"].as_str() == Some(&players[0].agent_id.to_string())).unwrap();
        assert_ne!(own["hole_cards"], json!(null));

        // Player 0 should NOT see player 1's cards
        let other = ps.iter().find(|p| p["agent_id"].as_str() == Some(&players[1].agent_id.to_string())).unwrap();
        assert_eq!(other["hole_cards"], json!(null));
    }

    #[test]
    fn test_is_terminal_only_on_completed() {
        let game = TexasHoldemV1;
        assert!(!game.is_terminal(&json!({"phase": "pre_flop"})));
        assert!(game.is_terminal(&json!({"phase": "completed"})));
    }

    #[test]
    fn test_two_player_full_game_fold() {
        let game = TexasHoldemV1;
        let players = make_players(2);
        let seed = [5u8; 32];
        let mut state = game.init(players.clone(), seed).unwrap();

        // Play out by folding until terminal
        let mut steps = 0;
        while !game.is_terminal(&state) {
            let action_seat = state["action_on_seat"].as_i64().unwrap();
            let acting = players
                .iter()
                .find(|p| p.seat_number == action_seat as i16)
                .unwrap();
            let actions = game.valid_actions(&state, acting.agent_id).unwrap();
            if actions.is_empty() { break; }
            let action = if actions.contains(&"fold".to_string()) { "fold" } else { &actions[0] };
            state = game.apply_action(state, acting.agent_id, action, None).unwrap();
            steps += 1;
            if steps > 100 { break; }
        }

        assert!(game.is_terminal(&state));
    }
}
