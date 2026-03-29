use crate::compact::{CompactGame, CompactSpec};
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};

const MAX_HAND_CARDS: usize = 12;
const DECK_SIZE: usize = 52;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BlackjackAction {
    Hit,
    Stand,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BlackjackPhase {
    PlayerTurn,
    OpponentTurn,
    Terminal,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HandValue {
    pub total: u8,
    pub soft: bool,
    pub busted: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlackjackState {
    deck: [u8; DECK_SIZE],
    next_card: u8,
    player_cards: [u8; MAX_HAND_CARDS],
    player_len: u8,
    opponent_cards: [u8; MAX_HAND_CARDS],
    opponent_len: u8,
    phase: BlackjackPhase,
    winner: Option<PlayerId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlackjackObservation {
    pub phase: BlackjackPhase,
    pub terminal: bool,
    pub winner: Option<PlayerId>,
    pub player_cards: [u8; MAX_HAND_CARDS],
    pub player_len: u8,
    pub player_value: HandValue,
    pub opponent_cards: [u8; MAX_HAND_CARDS],
    pub opponent_visible_len: u8,
    pub opponent_card_count: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlackjackSpectatorObservation {
    pub phase: BlackjackPhase,
    pub terminal: bool,
    pub winner: Option<PlayerId>,
    pub player_cards: [u8; MAX_HAND_CARDS],
    pub player_len: u8,
    pub player_value: HandValue,
    pub opponent_cards: [u8; MAX_HAND_CARDS],
    pub opponent_len: u8,
    pub opponent_value: HandValue,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Blackjack;

impl Blackjack {
    fn evaluate_hand(cards: &[u8], len: u8) -> HandValue {
        let mut total = 0u8;
        let mut aces = 0u8;
        for &card in cards.iter().take(len as usize) {
            match card {
                1 => {
                    total = total.saturating_add(11);
                    aces += 1;
                }
                11..=13 => total = total.saturating_add(10),
                value => total = total.saturating_add(value),
            }
        }
        while total > 21 && aces > 0 {
            total -= 10;
            aces -= 1;
        }
        HandValue {
            total,
            soft: aces > 0,
            busted: total > 21,
        }
    }

    fn fill_deck(deck: &mut [u8; DECK_SIZE]) {
        let mut index = 0usize;
        for _ in 0..4 {
            for rank in 1..=13 {
                deck[index] = rank;
                index += 1;
            }
        }
    }

    fn draw_card(state: &mut BlackjackState) -> u8 {
        let card = state.deck[state.next_card as usize];
        state.next_card += 1;
        card
    }

    fn push_player_card(state: &mut BlackjackState, card: u8) {
        state.player_cards[state.player_len as usize] = card;
        state.player_len += 1;
    }

    fn push_opponent_card(state: &mut BlackjackState, card: u8) {
        state.opponent_cards[state.opponent_len as usize] = card;
        state.opponent_len += 1;
    }

    fn player_value(state: &BlackjackState) -> HandValue {
        Self::evaluate_hand(&state.player_cards, state.player_len)
    }

    fn opponent_value(state: &BlackjackState) -> HandValue {
        Self::evaluate_hand(&state.opponent_cards, state.opponent_len)
    }

    fn resolve_terminal(state: &mut BlackjackState) -> i64 {
        let player = Self::player_value(state);
        let opponent = Self::opponent_value(state);
        state.phase = BlackjackPhase::Terminal;
        let (reward, winner) = if player.busted {
            (-1, Some(1))
        } else if opponent.busted || player.total > opponent.total {
            (1, Some(0))
        } else if player.total < opponent.total {
            (-1, Some(1))
        } else {
            (0, None)
        };
        state.winner = winner;
        reward
    }

    fn resolve_opponent_turn(state: &mut BlackjackState, rng: &mut DeterministicRng) -> i64 {
        state.phase = BlackjackPhase::OpponentTurn;
        loop {
            let value = Self::opponent_value(state);
            if value.busted || value.total == 21 {
                break;
            }
            let hit = rng.gen_range(2) == 0;
            if !hit {
                break;
            }
            let card = Self::draw_card(state);
            Self::push_opponent_card(state, card);
        }
        Self::resolve_terminal(state)
    }

    fn pack_cards(cards: &[u8; MAX_HAND_CARDS], len: u8) -> u64 {
        let mut packed = 0u64;
        for (index, &card) in cards.iter().take(len as usize).enumerate() {
            packed |= u64::from(card) << (index * 4);
        }
        packed
    }
}

impl Game for Blackjack {
    type State = BlackjackState;
    type Action = BlackjackAction;
    type PlayerObservation = BlackjackObservation;
    type SpectatorObservation = BlackjackSpectatorObservation;

    fn name(&self) -> &'static str {
        "blackjack"
    }

    fn player_count(&self) -> usize {
        1
    }

    fn init(&self, seed: Seed) -> Self::State {
        let mut rng = DeterministicRng::from_seed_and_stream(seed, 0);
        let mut deck = [0u8; DECK_SIZE];
        Self::fill_deck(&mut deck);
        rng.shuffle(&mut deck);

        let mut state = BlackjackState {
            deck,
            next_card: 0,
            player_cards: [0; MAX_HAND_CARDS],
            player_len: 0,
            opponent_cards: [0; MAX_HAND_CARDS],
            opponent_len: 0,
            phase: BlackjackPhase::PlayerTurn,
            winner: None,
        };

        let player_card_1 = Self::draw_card(&mut state);
        Self::push_player_card(&mut state, player_card_1);
        let opponent_card_1 = Self::draw_card(&mut state);
        Self::push_opponent_card(&mut state, opponent_card_1);
        let player_card_2 = Self::draw_card(&mut state);
        Self::push_player_card(&mut state, player_card_2);
        let opponent_card_2 = Self::draw_card(&mut state);
        Self::push_opponent_card(&mut state, opponent_card_2);
        state
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(state.phase, BlackjackPhase::Terminal)
    }

    fn players_to_act(&self, state: &Self::State, out: &mut Vec<PlayerId>) {
        out.clear();
        if !self.is_terminal(state) {
            out.push(0);
        }
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Vec<Self::Action>) {
        out.clear();
        if player != 0 || self.is_terminal(state) {
            return;
        }
        let value = Self::player_value(state);
        if value.total >= 21 {
            out.push(BlackjackAction::Stand);
        } else {
            out.push(BlackjackAction::Hit);
            out.push(BlackjackAction::Stand);
        }
    }

    fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::PlayerObservation {
        let terminal = self.is_terminal(state);
        let opponent_visible_len = if terminal { state.opponent_len } else { 0 };
        let mut opponent_cards = [0u8; MAX_HAND_CARDS];
        if terminal {
            opponent_cards = state.opponent_cards;
        }
        BlackjackObservation {
            phase: state.phase,
            terminal,
            winner: state.winner,
            player_cards: state.player_cards,
            player_len: state.player_len,
            player_value: Self::player_value(state),
            opponent_cards,
            opponent_visible_len,
            opponent_card_count: state.opponent_len,
        }
    }

    fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation {
        BlackjackSpectatorObservation {
            phase: state.phase,
            terminal: self.is_terminal(state),
            winner: state.winner,
            player_cards: state.player_cards,
            player_len: state.player_len,
            player_value: Self::player_value(state),
            opponent_cards: state.opponent_cards,
            opponent_len: state.opponent_len,
            opponent_value: Self::opponent_value(state),
        }
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &[PlayerAction<Self::Action>],
        rng: &mut DeterministicRng,
        out: &mut StepOutcome,
    ) {
        let action = joint_actions
            .iter()
            .find(|candidate| candidate.player == 0)
            .map(|candidate| candidate.action);

        let reward = if self.is_terminal(state) {
            out.termination = Termination::Terminal {
                winner: state.winner,
            };
            0
        } else if let Some(action) = action {
            let player_value = Self::player_value(state);
            let legal = if player_value.total >= 21 {
                matches!(action, BlackjackAction::Stand)
            } else {
                true
            };
            if !legal {
                state.phase = BlackjackPhase::Terminal;
                state.winner = Some(1);
                out.termination = Termination::Terminal { winner: Some(1) };
                -1
            } else {
                match action {
                    BlackjackAction::Hit => {
                        let card = Self::draw_card(state);
                        Self::push_player_card(state, card);
                        let updated = Self::player_value(state);
                        if updated.busted {
                            state.phase = BlackjackPhase::Terminal;
                            state.winner = Some(1);
                            out.termination = Termination::Terminal { winner: Some(1) };
                            -1
                        } else if updated.total == 21 {
                            let reward = Self::resolve_opponent_turn(state, rng);
                            out.termination = Termination::Terminal {
                                winner: state.winner,
                            };
                            reward
                        } else {
                            0
                        }
                    }
                    BlackjackAction::Stand => {
                        let reward = Self::resolve_opponent_turn(state, rng);
                        out.termination = Termination::Terminal {
                            winner: state.winner,
                        };
                        reward
                    }
                }
            }
        } else {
            state.phase = BlackjackPhase::Terminal;
            state.winner = Some(1);
            out.termination = Termination::Terminal { winner: Some(1) };
            -1
        };

        out.rewards.push(PlayerReward { player: 0, reward });
        if !self.is_terminal(state) {
            out.termination = Termination::Ongoing;
        }
    }
}

impl CompactGame for Blackjack {
    fn compact_spec(&self) -> CompactSpec {
        CompactSpec {
            action_count: 2,
            observation_bits: 64,
            observation_stream_len: 4,
            reward_bits: 2,
            min_reward: -1,
            max_reward: 1,
            reward_offset: 1,
        }
    }

    fn encode_action(&self, action: &Self::Action) -> u64 {
        match action {
            BlackjackAction::Hit => 0,
            BlackjackAction::Stand => 1,
        }
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        match encoded {
            0 => Some(BlackjackAction::Hit),
            1 => Some(BlackjackAction::Stand),
            _ => None,
        }
    }

    fn encode_player_observation(&self, observation: &Self::PlayerObservation, out: &mut Vec<u64>) {
        out.clear();
        let winner_code = match observation.winner {
            None => 0,
            Some(0) => 1,
            Some(_) => 2,
        };
        let phase = match observation.phase {
            BlackjackPhase::PlayerTurn => 0u64,
            BlackjackPhase::OpponentTurn => 1,
            BlackjackPhase::Terminal => 2,
        };
        let header = phase
            | ((observation.terminal as u64) << 4)
            | ((u64::from(observation.player_len)) << 8)
            | ((u64::from(observation.player_value.total)) << 12)
            | ((observation.player_value.soft as u64) << 20)
            | ((u64::from(observation.opponent_card_count)) << 24)
            | ((u64::from(observation.opponent_visible_len)) << 28)
            | ((winner_code as u64) << 32);
        out.push(header);
        out.push(Self::pack_cards(
            &observation.player_cards,
            observation.player_len,
        ));
        out.push(Self::pack_cards(
            &observation.opponent_cards,
            observation.opponent_visible_len,
        ));
        out.push(0);
    }

    fn encode_spectator_observation(
        &self,
        observation: &Self::SpectatorObservation,
        out: &mut Vec<u64>,
    ) {
        out.clear();
        let winner_code = match observation.winner {
            None => 0,
            Some(0) => 1,
            Some(_) => 2,
        };
        let phase = match observation.phase {
            BlackjackPhase::PlayerTurn => 0u64,
            BlackjackPhase::OpponentTurn => 1,
            BlackjackPhase::Terminal => 2,
        };
        let header = phase
            | ((observation.terminal as u64) << 4)
            | ((u64::from(observation.player_len)) << 8)
            | ((u64::from(observation.player_value.total)) << 12)
            | ((observation.player_value.soft as u64) << 20)
            | ((u64::from(observation.opponent_len)) << 24)
            | ((u64::from(observation.opponent_value.total)) << 28)
            | ((winner_code as u64) << 36);
        out.push(header);
        out.push(Self::pack_cards(
            &observation.player_cards,
            observation.player_len,
        ));
        out.push(Self::pack_cards(
            &observation.opponent_cards,
            observation.opponent_len,
        ));
        out.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    fn state_from_hands(player: &[u8], opponent: &[u8]) -> BlackjackState {
        let mut state = BlackjackState {
            deck: [0; DECK_SIZE],
            next_card: 0,
            player_cards: [0; MAX_HAND_CARDS],
            player_len: 0,
            opponent_cards: [0; MAX_HAND_CARDS],
            opponent_len: 0,
            phase: BlackjackPhase::PlayerTurn,
            winner: None,
        };
        for &card in player {
            Blackjack::push_player_card(&mut state, card);
        }
        for &card in opponent {
            Blackjack::push_opponent_card(&mut state, card);
        }
        state
    }

    #[test]
    fn hand_value_handles_soft_aces() {
        assert_eq!(
            Blackjack::evaluate_hand(&[1, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 2),
            HandValue {
                total: 21,
                soft: true,
                busted: false,
            }
        );
        assert_eq!(
            Blackjack::evaluate_hand(&[1, 1, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0], 3),
            HandValue {
                total: 21,
                soft: true,
                busted: false,
            }
        );
        assert_eq!(
            Blackjack::evaluate_hand(&[1, 1, 10, 10, 0, 0, 0, 0, 0, 0, 0, 0], 4),
            HandValue {
                total: 22,
                soft: false,
                busted: true,
            }
        );
    }

    #[test]
    fn shuffled_deck_is_a_full_permutation() {
        let state = Blackjack.init(11);
        let mut counts = [0u8; 14];
        for card in state.deck {
            counts[card as usize] += 1;
        }
        for rank in 1..=13 {
            assert_eq!(counts[rank], 4, "rank {rank} should appear four times");
        }
    }

    #[test]
    fn showdown_matrix_is_correct() {
        let mut player_win = state_from_hands(&[10, 10], &[9, 9]);
        assert_eq!(Blackjack::resolve_terminal(&mut player_win), 1);
        assert_eq!(player_win.winner, Some(0));

        let mut opponent_win = state_from_hands(&[10, 8], &[10, 9]);
        assert_eq!(Blackjack::resolve_terminal(&mut opponent_win), -1);
        assert_eq!(opponent_win.winner, Some(1));

        let mut push = state_from_hands(&[10, 7], &[9, 8]);
        assert_eq!(Blackjack::resolve_terminal(&mut push), 0);
        assert_eq!(push.winner, None);
    }

    #[test]
    fn seeded_round_trip_is_reproducible() {
        let actions = [PlayerAction {
            player: 0,
            action: BlackjackAction::Stand,
        }];
        let mut left = Session::new(Blackjack, 11);
        let mut right = Session::new(Blackjack, 11);
        left.step(&actions);
        right.step(&actions);
        assert_eq!(left.trace(), right.trace());
        assert_eq!(left.state(), right.state());
    }
}
