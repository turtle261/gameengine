use vstd::prelude::*;

verus! {

pub trait KernelReplayModel {
    type State;
    type Action;

    spec fn init(seed: nat) -> Self::State;
    spec fn step(state: Self::State, action: Self::Action) -> Self::State;
    spec fn replay(seed: nat, actions: Seq<Self::Action>) -> Self::State;
    spec fn replay_from(state: Self::State, actions: Seq<Self::Action>) -> Self::State;

    proof fn replay_from_empty_axiom(state: Self::State)
        ensures
            Self::replay_from(state, Seq::<Self::Action>::empty()) == state;

    proof fn replay_from_step_axiom(
        state: Self::State,
        prefix: Seq<Self::Action>,
        next: Self::Action,
    )
        ensures
            Self::replay_from(state, prefix.push(next))
                == Self::step(Self::replay_from(state, prefix), next);

    proof fn replay_is_from_init_axiom(seed: nat, actions: Seq<Self::Action>)
        ensures
            Self::replay(seed, actions) == Self::replay_from(Self::init(seed), actions);
}

pub proof fn replay_empty_refines_init<M: KernelReplayModel>(seed: nat)
    ensures
        M::replay(seed, Seq::<M::Action>::empty()) == M::init(seed),
{
    M::replay_is_from_init_axiom(seed, Seq::<M::Action>::empty());
    M::replay_from_empty_axiom(M::init(seed));
}

pub proof fn replay_refines_left_fold<M: KernelReplayModel>(
    seed: nat,
    prefix: Seq<M::Action>,
    next: M::Action,
)
    ensures
        M::replay(seed, prefix.push(next)) == M::step(M::replay(seed, prefix), next),
{
    M::replay_is_from_init_axiom(seed, prefix.push(next));
    M::replay_from_step_axiom(M::init(seed), prefix, next);
    M::replay_is_from_init_axiom(seed, prefix);
}

pub proof fn replay_singleton_refines_one_step<M: KernelReplayModel>(
    seed: nat,
    action: M::Action,
)
    ensures
        M::replay(seed, Seq::<M::Action>::empty().push(action))
            == M::step(M::init(seed), action),
{
    replay_refines_left_fold::<M>(seed, Seq::<M::Action>::empty(), action);
    replay_empty_refines_init::<M>(seed);
}

pub proof fn replay_from_prefix_state_refines_left_fold<M: KernelReplayModel>(
    seed: nat,
    prefix: Seq<M::Action>,
    suffix_prefix: Seq<M::Action>,
    next: M::Action,
)
    ensures
        M::replay_from(M::replay(seed, prefix), suffix_prefix.push(next))
            == M::step(M::replay_from(M::replay(seed, prefix), suffix_prefix), next),
{
    M::replay_from_step_axiom(M::replay(seed, prefix), suffix_prefix, next);
}

pub trait ObservationModel {
    type State;
    type Obs;

    spec fn observe(state: Self::State, who: int) -> Self::Obs;
    spec fn observer_is_valid(who: int) -> bool;
    spec fn obs_well_formed(obs: Self::Obs) -> bool;
    spec fn obs_schema_id(obs: Self::Obs) -> nat;
    spec fn canonical_schema_id() -> nat;

    proof fn observation_totality_axiom(state: Self::State, who: int)
        requires
            Self::observer_is_valid(who)
        ensures
            Self::obs_well_formed(Self::observe(state, who)),
            Self::obs_schema_id(Self::observe(state, who)) == Self::canonical_schema_id();
}

pub proof fn canonical_observation_schema_for_any_view<M: ObservationModel>(
    state: M::State,
    who_a: int,
    who_b: int,
)
    requires
        M::observer_is_valid(who_a),
        M::observer_is_valid(who_b)
    ensures
        M::obs_well_formed(M::observe(state, who_a)),
        M::obs_well_formed(M::observe(state, who_b)),
        M::obs_schema_id(M::observe(state, who_a)) == M::obs_schema_id(M::observe(state, who_b)),
{
    M::observation_totality_axiom(state, who_a);
    M::observation_totality_axiom(state, who_b);
}

pub proof fn canonical_schema_matches_declared_id<M: ObservationModel>(state: M::State, who: int)
    requires
        M::observer_is_valid(who)
    ensures
        M::obs_well_formed(M::observe(state, who)),
        M::obs_schema_id(M::observe(state, who)) == M::canonical_schema_id(),
{
    M::observation_totality_axiom(state, who);
}

} // verus!
