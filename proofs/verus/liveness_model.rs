use vstd::prelude::*;

verus! {

pub trait RankedTransitionModel {
    type State;
    type Action;

    spec fn step(state: Self::State, action: Self::Action) -> Self::State;
    spec fn terminal(state: Self::State) -> bool;
    spec fn rank(state: Self::State) -> nat;

    proof fn terminal_rank_axiom(state: Self::State)
        ensures
            Self::terminal(state) <==> Self::rank(state) == 0;

    proof fn rank_decreases_axiom(state: Self::State, action: Self::Action)
        requires
            !Self::terminal(state)
        ensures
            Self::terminal(Self::step(state, action))
                || Self::rank(Self::step(state, action)) < Self::rank(state);
}

pub proof fn ranked_progress_is_well_founded<M: RankedTransitionModel>(
    state: M::State,
    action: M::Action,
)
    requires
        !M::terminal(state)
    ensures
        M::terminal(M::step(state, action)) || M::rank(M::step(state, action)) < M::rank(state),
{
    M::rank_decreases_axiom(state, action);
}

pub trait FiniteSupportModel {
    type State;
    type Action;

    spec fn support(state: Self::State, action: Self::Action) -> Seq<(nat, Self::State)>;

    proof fn support_nonempty_axiom(state: Self::State, action: Self::Action)
        ensures
            Self::support(state, action).len() > 0;

    proof fn support_positive_weights_axiom(state: Self::State, action: Self::Action, index: int)
        requires
            0 <= index < Self::support(state, action).len()
        ensures
            Self::support(state, action)[index].0 > 0;
}

pub proof fn finite_support_has_positive_mass<M: FiniteSupportModel>(
    state: M::State,
    action: M::Action,
)
    ensures
        M::support(state, action).len() > 0,
{
    M::support_nonempty_axiom(state, action);
}

} // verus!
