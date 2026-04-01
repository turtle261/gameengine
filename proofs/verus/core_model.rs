use vstd::prelude::*;

verus! {

pub trait DeterministicTransition {
    type State;
    type Action;

    spec fn step(state: Self::State, action: Self::Action) -> Self::State;
}

pub proof fn deterministic_step_reflexive<T: DeterministicTransition>(
    state: T::State,
    action: T::Action,
)
    ensures
        T::step(state, action) == T::step(state, action),
{
}

pub trait ReplayModel {
    type State;
    type Action;

    spec fn init() -> Self::State;
    spec fn apply(state: Self::State, action: Self::Action) -> Self::State;
    spec fn replay(log: Seq<Self::Action>) -> Self::State;

    proof fn replay_prefix_axiom(log: Seq<Self::Action>, next: Self::Action)
        ensures
            Self::replay(log.push(next)) == Self::apply(Self::replay(log), next);
}

pub proof fn replay_prefix_is_refinement<T: ReplayModel>(log: Seq<T::Action>, next: T::Action)
    ensures
        T::replay(log.push(next)) == T::apply(T::replay(log), next),
{
    T::replay_prefix_axiom(log, next);
}

} // verus!
