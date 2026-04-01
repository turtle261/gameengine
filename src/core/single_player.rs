//! Reusable helpers for deterministic single-player environments.

use crate::buffer::Buffer;
use crate::types::{PlayerAction, PlayerId, PlayerReward, Reward};

/// Canonical acting player id used by single-player environments.
pub const SOLO_PLAYER: PlayerId = 0;

/// Returns true when `player` can act in a non-terminal single-player state.
pub const fn can_act(player: PlayerId, terminal: bool) -> bool {
    player == SOLO_PLAYER && !terminal
}

/// Clears and emits the single acting player when the state is ongoing.
pub fn write_players_to_act<B>(out: &mut B, terminal: bool)
where
    B: Buffer<Item = PlayerId>,
{
    out.clear();
    if !terminal {
        out.push(SOLO_PLAYER).unwrap();
    }
}

/// Returns the first action assigned to the single acting player.
pub fn first_action<A: Copy>(joint_actions: &[PlayerAction<A>]) -> Option<A> {
    for candidate in joint_actions {
        if candidate.player == SOLO_PLAYER {
            return Some(candidate.action);
        }
    }
    None
}

/// Appends one reward entry for the single acting player.
pub fn push_reward<B>(out: &mut B, reward: Reward)
where
    B: Buffer<Item = PlayerReward>,
{
    out.push(PlayerReward {
        player: SOLO_PLAYER,
        reward,
    })
    .unwrap();
}
