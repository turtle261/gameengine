use crate::buffer::FixedVec;
use crate::compact::{CompactGame, CompactSpec};
use crate::game::Game;
use crate::rng::DeterministicRng;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};
const MAX_HAND_CARDS: usize = 12;
const DECK_SIZE: usize = 52;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BlackjackAction {
    #[default]
    Hit,
    Stand,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BlackjackPhase {
    #[default]
    PlayerTurn,
    OpponentTurn,
    Terminal,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct HandValue {
    pub total: u8,
    pub soft: bool,
    pub busted: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlackjackState {
    pub deck: [u8; DECK_SIZE],
    pub next_card: u8,
    pub player_cards: [u8; MAX_HAND_CARDS],
    pub player_len: u8,
    pub opponent_cards: [u8; MAX_HAND_CARDS],
    pub opponent_len: u8,
    pub phase: BlackjackPhase,
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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
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

pub type BlackjackWorldView = BlackjackSpectatorObservation;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Blackjack;

impl Blackjack {
    fn evaluate_hand(cards: &[u8], len: u8) -> HandValue {
        let mut total = 0u8;
        let mut aces = 0u8;
        let limit = len as usize;
        let mut index = 0usize;
        let max_len = MAX_HAND_CARDS.min(cards.len());
        while index < max_len {
            if index >= limit {
                break;
            }
            let card = cards[index];
            match card {
                1 => {
                    total = total.saturating_add(11);
                    aces += 1;
                }
                11..=13 => total = total.saturating_add(10),
                value => total = total.saturating_add(value),
            }
            index += 1;
        }
        for _ in 0..MAX_HAND_CARDS {
            if total <= 21 || aces == 0 {
                break;
            }
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
            if state.next_card as usize >= DECK_SIZE {
                break;
            }
            let card = Self::draw_card(state);
            Self::push_opponent_card(state, card);
        }
        Self::resolve_terminal(state)
    }

    fn pack_cards(cards: &[u8; MAX_HAND_CARDS], len: u8) -> u64 {
        let mut packed = 0u64;
        let limit = len as usize;
        let mut index = 0usize;
        while index < MAX_HAND_CARDS {
            if index >= limit {
                break;
            }
            packed |= u64::from(cards[index]) << (index * 4);
            index += 1;
        }
        packed
    }
}

impl Game for Blackjack {
    type State = BlackjackState;
    type Action = BlackjackAction;
    type PlayerObservation = BlackjackObservation;
    type SpectatorObservation = BlackjackSpectatorObservation;
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
        out.clear();
        if !self.is_terminal(state) {
            out.push(0).unwrap();
        }
    }

    fn legal_actions(&self, state: &Self::State, player: PlayerId, out: &mut Self::ActionBuf) {
        out.clear();
        if player != 0 || self.is_terminal(state) {
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
        let actions = joint_actions.as_slice();
        let mut action = None;
        let mut index = 0usize;
        while index < actions.len() {
            let candidate = &actions[index];
            if candidate.player == 0 {
                action = Some(candidate.action);
                break;
            }
            index += 1;
        }

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

        out.rewards
            .push(PlayerReward { player: 0, reward })
            .unwrap();
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
        {
            return false;
        }
        let mut counts = [0u8; 14];
        for index in 0..DECK_SIZE {
            let card = state.deck[index];
            if !(1..=13).contains(&card) {
                return false;
            }
            counts[card as usize] += 1;
        }
        let mut rank = 1usize;
        while rank <= 13 {
            if counts[rank] != 4 {
                return false;
            }
            rank += 1;
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
        matches!(outcome.reward_for(0), -1..=1)
            && (post.phase == BlackjackPhase::Terminal) == outcome.is_terminal()
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

    fn encode_player_observation(
        &self,
        observation: &Self::PlayerObservation,
        out: &mut Self::WordBuf,
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
            | ((u64::from(observation.opponent_card_count)) << 24)
            | ((u64::from(observation.opponent_visible_len)) << 28)
            | ((winner_code as u64) << 32);
        out.push(header).unwrap();
        out.push(Self::pack_cards(
            &observation.player_cards,
            observation.player_len,
        ))
        .unwrap();
        out.push(Self::pack_cards(
            &observation.opponent_cards,
            observation.opponent_visible_len,
        ))
        .unwrap();
        out.push(0).unwrap();
    }

    fn encode_spectator_observation(
        &self,
        observation: &Self::SpectatorObservation,
        out: &mut Self::WordBuf,
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
        out.push(header).unwrap();
        out.push(Self::pack_cards(
            &observation.player_cards,
            observation.player_len,
        ))
        .unwrap();
        out.push(Self::pack_cards(
            &observation.opponent_cards,
            observation.opponent_len,
        ))
        .unwrap();
        out.push(0).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{FirstLegalPolicy, RandomPolicy};
    use crate::session::Session;
    use crate::verification::{
        assert_compact_roundtrip, assert_observation_contracts, assert_transition_contracts,
    };

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
        Blackjack::fill_deck(&mut state.deck);
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
        let mut rank = 1usize;
        while rank <= 13 {
            assert_eq!(counts[rank], 4, "rank {rank} should appear four times");
            rank += 1;
        }
        assert_observation_contracts(&Blackjack, &state);
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
        let mut left = Session::new(Blackjack, 11);
        let mut right = Session::new(Blackjack, 11);
        let action = [PlayerAction {
            player: 0,
            action: BlackjackAction::Hit,
        }];
        let left_outcome = left.step(&action).clone();
        let right_outcome = right.step(&action).clone();
        assert_eq!(left.state(), right.state());
        assert_eq!(left_outcome, right_outcome);
    }

    #[test]
    fn verification_helpers_hold_for_player_hit() {
        let game = Blackjack;
        let state = game.init(11);
        let mut actions = FixedVec::<PlayerAction<BlackjackAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: BlackjackAction::Hit,
            })
            .unwrap();
        assert_transition_contracts(&game, &state, &actions, 11);
        assert_compact_roundtrip(&game, &BlackjackAction::Hit);
    }

    #[test]
    fn seeded_sessions_preserve_invariants_across_policies() {
        for seed in 1..=256 {
            let mut first = FirstLegalPolicy;
            let mut random = RandomPolicy;

            let mut first_session = Session::new(Blackjack, seed);
            assert!(Blackjack.state_invariant(first_session.state()));
            let mut first_policies: [&mut dyn crate::policy::Policy<Blackjack>; 1] = [&mut first];
            while !first_session.is_terminal() && first_session.current_tick() < 16 {
                first_session.step_with_policies(&mut first_policies);
            }
            assert!(Blackjack.state_invariant(first_session.state()));

            let mut random_session = Session::new(Blackjack, seed);
            assert!(Blackjack.state_invariant(random_session.state()));
            let mut random_policies: [&mut dyn crate::policy::Policy<Blackjack>; 1] = [&mut random];
            while !random_session.is_terminal() && random_session.current_tick() < 16 {
                random_session.step_with_policies(&mut random_policies);
            }
            assert!(Blackjack.state_invariant(random_session.state()));
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::{Blackjack, BlackjackAction, BlackjackPhase, HandValue, MAX_HAND_CARDS};
    use crate::buffer::FixedVec;
    use crate::game::Game;
    use crate::types::PlayerAction;

    #[kani::proof]
    #[kani::unwind(64)]
    fn concrete_seed_shuffle_is_a_full_permutation() {
        let state = Blackjack.init(11);
        let mut counts = [0u8; 14];
        for card in state.deck {
            counts[card as usize] += 1;
        }
        let mut rank = 1usize;
        while rank <= 13 {
            assert_eq!(counts[rank], 4);
            rank += 1;
        }
    }

    #[kani::proof]
    #[kani::unwind(64)]
    fn player_observation_hides_opponent_hand_before_terminal() {
        let state = Blackjack.init(11);
        let observation = Blackjack.observe_player(&state, 0);
        if state.phase != BlackjackPhase::Terminal {
            assert_eq!(observation.opponent_visible_len, 0);
        }
    }

    #[kani::proof]
    #[kani::unwind(64)]
    fn initial_observation_contracts_hold_for_concrete_seed() {
        let game = Blackjack;
        let state = game.init(11);
        crate::verification::assert_observation_contracts(&game, &state);
    }

    #[kani::proof]
    #[kani::unwind(64)]
    fn stand_action_replays_deterministically_for_seed_17() {
        let state = Blackjack.init(17);
        let mut actions = FixedVec::<PlayerAction<BlackjackAction>, 1>::default();
        actions
            .push(PlayerAction {
                player: 0,
                action: BlackjackAction::Stand,
            })
            .unwrap();
        crate::verification::assert_transition_contracts(&Blackjack, &state, &actions, 17);
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn hand_evaluation_matches_busted_flag() {
        let len: u8 = kani::any();
        kani::assume(len <= MAX_HAND_CARDS as u8);
        let mut cards = [1u8; MAX_HAND_CARDS];
        for card in &mut cards {
            *card = kani::any();
            kani::assume((1..=13).contains(card));
        }
        let value = Blackjack::evaluate_hand(&cards, len);
        assert_eq!(
            value,
            HandValue {
                total: value.total,
                soft: value.soft,
                busted: value.total > 21,
            }
        );
    }
}
