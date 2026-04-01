//! Builtin deterministic blackjack environment and compact observation codecs.

use crate::buffer::FixedVec;
use crate::compact::{CompactSpec, decode_enum_action, encode_enum_action};
use crate::core::cards::{
    BlackjackValue, evaluate_blackjack_hand, fill_standard_deck_52,
    is_standard_deck_52_permutation, pack_cards_nibbles,
};
use crate::core::single_player;
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};
use crate::verification::reward_and_terminal_postcondition;
const MAX_HAND_CARDS: usize = 12;
const DECK_SIZE: usize = 52;
const BLACKJACK_ACTION_ORDER: [BlackjackAction; 2] = [BlackjackAction::Hit, BlackjackAction::Stand];

/// Player action in the blackjack round.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BlackjackAction {
    /// Draw one additional card.
    #[default]
    Hit,
    /// End the player turn and let the opponent resolve.
    Stand,
}

/// High-level stage of a blackjack round.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BlackjackPhase {
    /// Waiting for the player-controlled action.
    #[default]
    PlayerTurn,
    /// Opponent policy is resolving draws.
    OpponentTurn,
    /// Round is completed.
    Terminal,
}

/// Evaluated value of a blackjack hand.
pub type HandValue = BlackjackValue;

/// Full deterministic blackjack state including shuffled deck.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlackjackState {
    /// Shuffled full deck represented as rank codes 1..=13.
    pub deck: [u8; DECK_SIZE],
    /// Index of the next card to draw from `deck`.
    pub next_card: u8,
    /// Player-held cards.
    pub player_cards: [u8; MAX_HAND_CARDS],
    /// Number of valid entries in `player_cards`.
    pub player_len: u8,
    /// Opponent-held cards.
    pub opponent_cards: [u8; MAX_HAND_CARDS],
    /// Number of valid entries in `opponent_cards`.
    pub opponent_len: u8,
    /// Current game phase.
    pub phase: BlackjackPhase,
    /// Winner id if terminal with a winner.
    pub winner: Option<PlayerId>,
}

impl Default for BlackjackState {
    fn default() -> Self {
        Self {
            deck: [0; DECK_SIZE],
            next_card: 0,
            player_cards: [0; MAX_HAND_CARDS],
            player_len: 0,
            opponent_cards: [0; MAX_HAND_CARDS],
            opponent_len: 0,
            phase: BlackjackPhase::PlayerTurn,
            winner: None,
        }
    }
}

/// Canonical blackjack observation shared across viewpoints.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BlackjackObservation {
    /// Current game phase.
    pub phase: BlackjackPhase,
    /// True if the round has completed.
    pub terminal: bool,
    /// Winner id if terminal with a winner.
    pub winner: Option<PlayerId>,
    /// Player cards visible to the observer.
    pub player_cards: [u8; MAX_HAND_CARDS],
    /// Number of valid entries in `player_cards`.
    pub player_len: u8,
    /// Evaluated player hand value.
    pub player_value: HandValue,
    /// Opponent cards visible to the observer.
    pub opponent_cards: [u8; MAX_HAND_CARDS],
    /// Number of valid entries in `opponent_cards` that are visible.
    pub opponent_visible_len: u8,
    /// Total opponent card count, including hidden cards.
    pub opponent_card_count: u8,
    /// Evaluated opponent hand value when available.
    pub opponent_value: HandValue,
}

/// Full world/debug view type.
pub type BlackjackWorldView = BlackjackObservation;

/// Builtin blackjack environment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Blackjack;

impl Blackjack {
    fn evaluate_hand(cards: &[u8], len: u8) -> HandValue {
        let mut hand = [0u8; MAX_HAND_CARDS];
        let max_len = MAX_HAND_CARDS.min(cards.len());
        hand[..max_len].copy_from_slice(&cards[..max_len]);
        evaluate_blackjack_hand(&hand, len)
    }

    fn fill_deck(deck: &mut [u8; DECK_SIZE]) {
        fill_standard_deck_52(deck);
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
            if state.next_card as usize >= DECK_SIZE {
                break;
            }
            let card = Self::draw_card(state);
            Self::push_opponent_card(state, card);
        }
        Self::resolve_terminal(state)
    }

    fn pack_cards(cards: &[u8; MAX_HAND_CARDS], len: u8) -> u64 {
        pack_cards_nibbles(cards, len)
    }

    fn winner_code(winner: Option<PlayerId>) -> u64 {
        match winner {
            None => 0,
            Some(0) => 1,
            Some(_) => 2,
        }
    }

    fn phase_code(phase: BlackjackPhase) -> u64 {
        match phase {
            BlackjackPhase::PlayerTurn => 0,
            BlackjackPhase::OpponentTurn => 1,
            BlackjackPhase::Terminal => 2,
        }
    }

    fn encode_observation_with_header(
        observation: &BlackjackObservation,
        header: u64,
        opponent_len: u8,
        out: &mut FixedVec<u64, 4>,
    ) {
        out.clear();
        out.push(header).unwrap();
        out.push(Self::pack_cards(
            &observation.player_cards,
            observation.player_len,
        ))
        .unwrap();
        out.push(Self::pack_cards(&observation.opponent_cards, opponent_len))
            .unwrap();
        out.push(0).unwrap();
    }
}

impl Game for Blackjack {
    type State = BlackjackState;
    type Action = BlackjackAction;
    type PlayerObservation = BlackjackObservation;
    type SpectatorObservation = BlackjackObservation;
    type WorldView = BlackjackWorldView;
    type PlayerBuf = FixedVec<PlayerId, 1>;
    type ActionBuf = FixedVec<BlackjackAction, 2>;
    type JointActionBuf = FixedVec<PlayerAction<BlackjackAction>, 1>;
    type RewardBuf = FixedVec<PlayerReward, 1>;
    type WordBuf = FixedVec<u64, 4>;

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

    fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
        single_player::write_players_to_act(out, self.is_terminal(state));
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf) {
        out.clear();
        if !single_player::can_act(player, self.is_terminal(state)) {
            return;
        }
        let value = Self::player_value(state);
        if value.total >= 21 {
            out.push(BlackjackAction::Stand).unwrap();
        } else {
            out.push(BlackjackAction::Hit).unwrap();
            out.push(BlackjackAction::Stand).unwrap();
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
            opponent_value: HandValue::default(),
        }
    }

    fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation {
        BlackjackObservation {
            phase: state.phase,
            terminal: self.is_terminal(state),
            winner: state.winner,
            player_cards: state.player_cards,
            player_len: state.player_len,
            player_value: Self::player_value(state),
            opponent_cards: state.opponent_cards,
            opponent_visible_len: state.opponent_len,
            opponent_card_count: state.opponent_len,
            opponent_value: Self::opponent_value(state),
        }
    }

    fn world_view(&self, state: &Self::State) -> Self::WorldView {
        self.observe_spectator(state)
    }

    fn step_in_place(
        &self,
        state: &mut Self::State,
        joint_actions: &Self::JointActionBuf,
        rng: &mut DeterministicRng,
        out: &mut StepOutcome<Self::RewardBuf>,
    ) {
        let action = single_player::first_action(joint_actions.as_slice());

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

        single_player::push_reward(&mut out.rewards, reward);
        if !self.is_terminal(state) {
            out.termination = Termination::Ongoing;
        }
    }

    fn state_invariant(&self, state: &Self::State) -> bool {
        if state.player_len < 2
            || state.opponent_len < 2
            || usize::from(state.player_len) > MAX_HAND_CARDS
            || usize::from(state.opponent_len) > MAX_HAND_CARDS
            || usize::from(state.next_card) > DECK_SIZE
            || !is_standard_deck_52_permutation(&state.deck)
        {
            return false;
        }
        if self.is_terminal(state) {
            let mut resolved = *state;
            Self::resolve_terminal(&mut resolved);
            resolved.winner == state.winner
        } else {
            true
        }
    }

    fn player_observation_invariant(
        &self,
        state: &Self::State,
        _player: PlayerId,
        observation: &Self::PlayerObservation,
    ) -> bool {
        if self.is_terminal(state) {
            observation.opponent_visible_len == state.opponent_len
                && observation.opponent_cards == state.opponent_cards
        } else {
            if observation.opponent_visible_len != 0 {
                return false;
            }
            for index in 0..MAX_HAND_CARDS {
                if observation.opponent_cards[index] != 0 {
                    return false;
                }
            }
            true
        }
    }

    fn transition_postcondition(
        &self,
        _pre: &Self::State,
        _actions: &Self::JointActionBuf,
        post: &Self::State,
        outcome: &StepOutcome<Self::RewardBuf>,
    ) -> bool {
        reward_and_terminal_postcondition(outcome.reward_for(0), -1, 1, post.phase == BlackjackPhase::Terminal, outcome.is_terminal())
    }

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
        encode_enum_action(*action, &BLACKJACK_ACTION_ORDER)
    }

    fn decode_action(&self, encoded: u64) -> Option<Self::Action> {
        decode_enum_action(encoded, &BLACKJACK_ACTION_ORDER)
    }

    fn encode_player_observation(
        &self,
        observation: &Self::PlayerObservation,
        out: &mut Self::WordBuf,
    ) {
        let header = Self::phase_code(observation.phase)
            | ((observation.terminal as u64) << 4)
            | ((u64::from(observation.player_len)) << 8)
            | ((u64::from(observation.player_value.total)) << 12)
            | ((observation.player_value.soft as u64) << 20)
            | ((u64::from(observation.opponent_card_count)) << 24)
            | ((u64::from(observation.opponent_visible_len)) << 28)
            | (Self::winner_code(observation.winner) << 32);
        Self::encode_observation_with_header(
            observation,
            header,
            observation.opponent_visible_len,
            out,
        );
    }

    fn encode_spectator_observation(
        &self,
        observation: &Self::SpectatorObservation,
        out: &mut Self::WordBuf,
    ) {
        let header = Self::phase_code(observation.phase)
            | ((observation.terminal as u64) << 4)
            | ((u64::from(observation.player_len)) << 8)
            | ((u64::from(observation.player_value.total)) << 12)
            | ((observation.player_value.soft as u64) << 20)
            | ((u64::from(observation.opponent_card_count)) << 24)
            | ((u64::from(observation.opponent_value.total)) << 28)
            | (Self::winner_code(observation.winner) << 36);
        Self::encode_observation_with_header(
            observation,
            header,
            observation.opponent_visible_len,
            out,
        );
    }

}

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
