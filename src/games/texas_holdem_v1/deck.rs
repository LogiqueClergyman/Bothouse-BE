use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rand::seq::SliceRandom;

pub fn make_deck() -> Vec<String> {
    let ranks = ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"];
    let suits = ["h", "d", "c", "s"];
    let mut deck = Vec::with_capacity(52);
    for rank in &ranks {
        for suit in &suits {
            deck.push(format!("{}{}", rank, suit));
        }
    }
    deck
}

pub fn shuffle_deck(seed: [u8; 32]) -> Vec<String> {
    let mut deck = make_deck();
    let mut rng = ChaCha20Rng::from_seed(seed);
    deck.shuffle(&mut rng);
    deck
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_deck_has_52_unique_cards() {
        let deck = make_deck();
        assert_eq!(deck.len(), 52);
        let unique: HashSet<_> = deck.iter().collect();
        assert_eq!(unique.len(), 52);
    }

    #[test]
    fn test_shuffle_is_deterministic() {
        let seed = [42u8; 32];
        let deck1 = shuffle_deck(seed);
        let deck2 = shuffle_deck(seed);
        assert_eq!(deck1, deck2);
    }

    #[test]
    fn test_different_seeds_produce_different_orders() {
        let seed1 = [1u8; 32];
        let seed2 = [2u8; 32];
        let deck1 = shuffle_deck(seed1);
        let deck2 = shuffle_deck(seed2);
        assert_ne!(deck1, deck2);
    }

    #[test]
    fn test_shuffled_deck_still_52_unique() {
        let deck = shuffle_deck([0u8; 32]);
        assert_eq!(deck.len(), 52);
        let unique: std::collections::HashSet<_> = deck.iter().collect();
        assert_eq!(unique.len(), 52);
    }
}
