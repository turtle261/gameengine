use gameengine::core::single_player::{self, SinglePlayerGame, SinglePlayerRewardBuf};
use gameengine::{
    Buffer, DeterministicRng, FixedVec, PlayerId, Seed, Session, StepOutcome, Termination,
};

const W: i16 = 40;
const H: i16 = 20;
const P: i16 = 2;
const WIN: u8 = 5;
const ACTIONS: [Act; 3] = [Act::Stay, Act::Up, Act::Down];

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
enum Act {
    #[default]
    Stay,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
struct St {
    p1: i16,
    p2: i16,
    bx: i16,
    by: i16,
    vx: i16,
    vy: i16,
    s1: u8,
    s2: u8,
    done: bool,
    winner: Option<PlayerId>,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
struct Pong;

impl Pong {
    fn clamp(y: i16) -> i16 {
        y.clamp(P, H - 1 - P)
    }
    fn reset_ball(st: &mut St, toward_p1: bool) {
        st.bx = W / 2;
        st.by = H / 2;
        st.vx = if toward_p1 { -1 } else { 1 };
        st.vy = if (st.s1 + st.s2).is_multiple_of(2) {
            1
        } else {
            -1
        };
    }
}

impl SinglePlayerGame for Pong {
    type Params = ();
    type State = St;
    type Action = Act;
    type Obs = St;
    type WorldView = St;
    type ActionBuf = FixedVec<Act, 3>;
    type WordBuf = FixedVec<u64, 1>;

    fn name(&self) -> &'static str {
        "pong-core"
    }
    fn init_with_params(&self, _seed: Seed, _params: &()) -> St {
        St {
            p1: H / 2,
            p2: H / 2,
            bx: W / 2,
            by: H / 2,
            vx: 1,
            vy: 1,
            ..St::default()
        }
    }
    fn is_terminal(&self, st: &St) -> bool {
        st.done
    }
    fn legal_actions(&self, _st: &St, out: &mut Self::ActionBuf) {
        out.clear();
        out.extend_from_slice(&ACTIONS).unwrap();
    }
    fn observe_player(&self, st: &St) -> St {
        *st
    }
    fn world_view(&self, st: &St) -> St {
        *st
    }
    fn step_in_place(
        &self,
        st: &mut St,
        action: Option<Act>,
        _rng: &mut DeterministicRng,
        out: &mut StepOutcome<SinglePlayerRewardBuf>,
    ) {
        if st.done {
            out.termination = Termination::Terminal { winner: st.winner };
            single_player::push_reward(&mut out.rewards, 0);
            return;
        }
        let dy = match action.unwrap_or(Act::Stay) {
            Act::Stay => 0,
            Act::Up => -1,
            Act::Down => 1,
        };
        st.p1 = Self::clamp(st.p1 + dy);
        st.p2 = Self::clamp(st.p2 + (st.by > st.p2) as i16 - (st.by < st.p2) as i16);
        st.bx += st.vx;
        st.by += st.vy;
        if st.by <= 0 || st.by >= H - 1 {
            st.by = st.by.clamp(0, H - 1);
            st.vy = -st.vy;
        }
        let mut reward = 0;
        if st.bx <= 1 && (st.by - st.p1).abs() <= P {
            st.vx = 1;
        } else if st.bx >= W - 2 && (st.by - st.p2).abs() <= P {
            st.vx = -1;
        } else if st.bx < 0 {
            st.s2 += 1;
            reward = -1;
            Self::reset_ball(st, false);
        } else if st.bx >= W {
            st.s1 += 1;
            reward = 1;
            Self::reset_ball(st, true);
        }
        if st.s1 >= WIN || st.s2 >= WIN {
            st.done = true;
            st.winner = Some(if st.s1 > st.s2 { 0 } else { 1 });
            out.termination = Termination::Terminal { winner: st.winner };
        } else {
            out.termination = Termination::Ongoing;
        }
        single_player::push_reward(&mut out.rewards, reward);
    }
}

fn main() {
    let mut session = Session::new(Pong, 7);
    while !session.is_terminal() && session.current_tick() < 64 {
        session.step(&[]);
    }
    println!(
        "tick={} score={} - {}",
        session.current_tick(),
        session.state().s1,
        session.state().s2
    );
}
