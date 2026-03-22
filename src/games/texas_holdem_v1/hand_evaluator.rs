use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRank {
    HighCard = 0,
    OnePair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
    RoyalFlush = 9,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandEvaluation {
    pub rank: HandRank,
    pub score: u32,
}

impl PartialOrd for HandEvaluation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HandEvaluation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

fn card_rank(card: &str) -> u8 {
    match &card[..card.len() - 1] {
        "2" => 2,
        "3" => 3,
        "4" => 4,
        "5" => 5,
        "6" => 6,
        "7" => 7,
        "8" => 8,
        "9" => 9,
        "T" => 10,
        "J" => 11,
        "Q" => 12,
        "K" => 13,
        "A" => 14,
        _ => 0,
    }
}

fn card_suit(card: &str) -> char {
    card.chars().last().unwrap_or('?')
}

/// Evaluate best 5-card hand from 5-7 cards.
/// Returns HandEvaluation with a score for comparison.
pub fn evaluate_best_hand(cards: &[String]) -> HandEvaluation {
    assert!(cards.len() >= 5, "Need at least 5 cards, got {}", cards.len());
    assert!(cards.len() <= 7);

    // Generate all C(n, 5) combinations
    let n = cards.len();
    let mut best: Option<HandEvaluation> = None;

    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                for l in (k + 1)..n {
                    for m in (l + 1)..n {
                        let hand = [
                            &cards[i],
                            &cards[j],
                            &cards[k],
                            &cards[l],
                            &cards[m],
                        ];
                        let eval = evaluate_five(&hand);
                        match &best {
                            None => best = Some(eval),
                            Some(b) if eval.score > b.score => best = Some(eval),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    best.unwrap()
}

fn evaluate_five(hand: &[&String; 5]) -> HandEvaluation {
    let mut ranks: Vec<u8> = hand.iter().map(|c| card_rank(c)).collect();
    let suits: Vec<char> = hand.iter().map(|c| card_suit(c)).collect();
    ranks.sort_unstable_by(|a, b| b.cmp(a));

    let is_flush = suits.iter().all(|&s| s == suits[0]);
    let is_straight = is_straight_check(&ranks);
    let is_wheel = ranks == [14, 5, 4, 3, 2]; // A-2-3-4-5

    if is_flush && !is_straight && !is_wheel {
        // Check royal flush / straight flush
        let is_sf = is_straight_check(&ranks);
        let is_wheel_flush = is_flush && is_wheel;
        if is_sf || is_wheel_flush {
            if ranks[0] == 14 && !is_wheel_flush {
                return HandEvaluation {
                    rank: HandRank::RoyalFlush,
                    score: score(HandRank::RoyalFlush, &ranks, false),
                };
            }
            return HandEvaluation {
                rank: HandRank::StraightFlush,
                score: score(HandRank::StraightFlush, &ranks, is_wheel),
            };
        }
        return HandEvaluation {
            rank: HandRank::Flush,
            score: score(HandRank::Flush, &ranks, false),
        };
    }

    if is_flush {
        // could still be straight flush
        if is_straight || is_wheel {
            if ranks[0] == 14 && !is_wheel {
                return HandEvaluation {
                    rank: HandRank::RoyalFlush,
                    score: score(HandRank::RoyalFlush, &ranks, false),
                };
            }
            return HandEvaluation {
                rank: HandRank::StraightFlush,
                score: score(HandRank::StraightFlush, &ranks, is_wheel),
            };
        }
        return HandEvaluation {
            rank: HandRank::Flush,
            score: score(HandRank::Flush, &ranks, false),
        };
    }

    if is_straight || is_wheel {
        return HandEvaluation {
            rank: HandRank::Straight,
            score: score(HandRank::Straight, &ranks, is_wheel),
        };
    }

    let counts = rank_counts(&ranks);
    let mut freq: Vec<(u8, u8)> = counts.into_iter().collect();
    freq.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

    match freq.as_slice() {
        [(_, 4), ..] => HandEvaluation {
            rank: HandRank::FourOfAKind,
            score: score(HandRank::FourOfAKind, &ranks, false),
        },
        [(_, 3), (_, 2), ..] => HandEvaluation {
            rank: HandRank::FullHouse,
            score: score(HandRank::FullHouse, &ranks, false),
        },
        [(_, 3), ..] => HandEvaluation {
            rank: HandRank::ThreeOfAKind,
            score: score(HandRank::ThreeOfAKind, &ranks, false),
        },
        [(_, 2), (_, 2), ..] => HandEvaluation {
            rank: HandRank::TwoPair,
            score: score(HandRank::TwoPair, &ranks, false),
        },
        [(_, 2), ..] => HandEvaluation {
            rank: HandRank::OnePair,
            score: score(HandRank::OnePair, &ranks, false),
        },
        _ => HandEvaluation {
            rank: HandRank::HighCard,
            score: score(HandRank::HighCard, &ranks, false),
        },
    }
}

fn is_straight_check(ranks: &[u8]) -> bool {
    // ranks assumed sorted desc
    if ranks.len() != 5 {
        return false;
    }
    let is_wheel = ranks == [14, 5, 4, 3, 2];
    if is_wheel {
        return false; // wheel handled separately
    }
    ranks[0] - ranks[4] == 4 && rank_counts(ranks).len() == 5
}

fn rank_counts(ranks: &[u8]) -> std::collections::HashMap<u8, u8> {
    let mut counts = std::collections::HashMap::new();
    for &r in ranks {
        *counts.entry(r).or_insert(0) += 1u8;
    }
    counts
}

/// Encode a score as: (hand_rank * 10^10) + kicker bits
/// Each card rank takes 4 bits (0-14). We encode 5 cards in priority order.
fn score(rank: HandRank, sorted_ranks: &[u8], is_wheel: bool) -> u32 {
    let base = (rank as u32) * 10_000_000;

    // Sort ranks by frequency then value for encoding
    let counts = rank_counts(sorted_ranks);
    let mut freq: Vec<(u8, u8)> = counts.into_iter().collect();
    freq.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

    let priority_ranks: Vec<u8> = freq.iter().map(|(r, _)| *r).collect();

    // Wheel (A-2-3-4-5) is the lowest possible straight.
    // Score it just above TwoPair so that Three of a Kind beats it.
    if is_wheel && matches!(rank, HandRank::Straight | HandRank::StraightFlush) {
        return (HandRank::TwoPair as u32) * 10_000_000 + 6;
    }

    // For straight/straight flush, use the high card
    let kicker = if matches!(rank, HandRank::Straight | HandRank::StraightFlush) {
        sorted_ranks[0] as u32
    } else {
        // Encode up to 5 ranks in priority order, 4 bits each
        let mut k = 0u32;
        for (i, &r) in priority_ranks.iter().take(5).enumerate() {
            k |= (r as u32) << ((4 - i) * 4);
        }
        k
    };

    base + kicker
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cards(s: &str) -> Vec<String> {
        s.split_whitespace().map(|c| c.to_string()).collect()
    }

    #[test]
    fn test_royal_flush() {
        let hand = evaluate_best_hand(&cards("Ah Kh Qh Jh Th 2c 3d"));
        assert_eq!(hand.rank, HandRank::RoyalFlush);
    }

    #[test]
    fn test_straight_flush() {
        let hand = evaluate_best_hand(&cards("9h 8h 7h 6h 5h 2c 3d"));
        assert_eq!(hand.rank, HandRank::StraightFlush);
    }

    #[test]
    fn test_four_of_a_kind() {
        let hand = evaluate_best_hand(&cards("As Ah Ad Ac 2c 3d 4s"));
        assert_eq!(hand.rank, HandRank::FourOfAKind);
    }

    #[test]
    fn test_full_house() {
        let hand = evaluate_best_hand(&cards("Ah Ad Ac Kh Ks 2c 3d"));
        assert_eq!(hand.rank, HandRank::FullHouse);
    }

    #[test]
    fn test_flush() {
        let hand = evaluate_best_hand(&cards("Ah 9h 7h 4h 2h 3c 5d"));
        assert_eq!(hand.rank, HandRank::Flush);
    }

    #[test]
    fn test_straight() {
        let hand = evaluate_best_hand(&cards("9h 8c 7d 6s 5h 2c 3d"));
        assert_eq!(hand.rank, HandRank::Straight);
    }

    #[test]
    fn test_wheel_straight() {
        let hand = evaluate_best_hand(&cards("Ah 2c 3d 4s 5h 9c Kd"));
        assert_eq!(hand.rank, HandRank::Straight);
    }

    #[test]
    fn test_three_of_a_kind() {
        let hand = evaluate_best_hand(&cards("Ah Ad Ac 2c 3d 4s 5h"));
        assert_eq!(hand.rank, HandRank::ThreeOfAKind);
    }

    #[test]
    fn test_two_pair() {
        let hand = evaluate_best_hand(&cards("Ah Ad Kh Kd 2c 3d 4s"));
        assert_eq!(hand.rank, HandRank::TwoPair);
    }

    #[test]
    fn test_one_pair() {
        let hand = evaluate_best_hand(&cards("Ah Ad 2c 3d 4s 6h 8c"));
        assert_eq!(hand.rank, HandRank::OnePair);
    }

    #[test]
    fn test_high_card() {
        let hand = evaluate_best_hand(&cards("Ah Kd Qc Js 9h 7c 2d"));
        assert_eq!(hand.rank, HandRank::HighCard);
    }

    #[test]
    fn test_better_pair_beats_worse_pair() {
        let aces = evaluate_best_hand(&cards("Ah Ad 2c 3d 4s 6h 8c"));
        let twos = evaluate_best_hand(&cards("2h 2d Kc Qd Js 9h 7c"));
        assert!(aces.score > twos.score);
    }

    #[test]
    fn test_kicker_comparison() {
        let ace_kicker = evaluate_best_hand(&cards("Kh Kd Ac 2d 3s 5h 7c"));
        let queen_kicker = evaluate_best_hand(&cards("Kh Kd Qc 2d 3s 5h 7c"));
        assert!(ace_kicker.score > queen_kicker.score);
    }

    #[test]
    fn test_tie_same_hand() {
        let hand1 = evaluate_best_hand(&cards("Ah Ad As 2c 3d 4s 5h"));
        let hand2 = evaluate_best_hand(&cards("Ah Ad As 2s 3s 4h 5d"));
        assert_eq!(hand1.score, hand2.score);
    }
}
