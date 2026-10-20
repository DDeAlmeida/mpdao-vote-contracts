use crate::*;
use near_sdk::json_types::U128;
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::testing_env;

fn acct(name: &str) -> AccountId {
    name.parse().unwrap()
}

const T0: u64 = 10 * 24 * 3600 * 1_000_000_000; // block timestamp in ns

fn context_at(predecessor: &str, timestamp_ns: u64) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(acct("metavote.near"))
        .predecessor_account_id(acct(predecessor))
        .block_timestamp(timestamp_ns);
    builder
}

fn context(predecessor: &str) -> VMContextBuilder {
    context_at(predecessor, T0)
}

fn new_contract() -> MetaVoteContract {
    testing_env!(context("owner.near").build());
    MetaVoteContract::new(
        acct("owner.near"),
        acct("operator.near"),
        30,
        300,
        U128(ONE_MPDAO),
        5,
        5,
        acct("mpdao.near"),
        acct("stnear.near"),
        U128(0),
        "prev.near".to_string(),
    )
}

/// voter with two locked positions: 100 mpDAO for 60 days (index 0) and 100 mpDAO for 120 days (index 1)
fn add_voter(contract: &mut MetaVoteContract, id: &str) {
    let voter_id = id.to_string();
    let mut voter = contract.internal_get_voter(&voter_id);
    contract.internal_create_locking_position(&mut voter, 100 * ONE_MPDAO, 60);
    contract.internal_create_locking_position(&mut voter, 100 * ONE_MPDAO, 120);
    contract.voters.insert(&voter_id, &voter);
}

fn vote_on(contract: &mut MetaVoteContract, app: &str, amount: u128) {
    testing_env!(context("alice.near").build());
    contract.vote(U128(amount), app.to_string(), "validator.near".to_string());
}

fn contract_with_alice() -> MetaVoteContract {
    // new_contract() configures max_voting_positions = 5
    let mut contract = new_contract();
    add_voter(&mut contract, "alice.near");
    contract
}

#[test]
fn a_voter_can_hold_up_to_the_configured_number_of_positions() {
    let mut contract = contract_with_alice();
    for app in ["app1.near", "app2.near", "app3.near", "app4.near", "app5.near"] {
        vote_on(&mut contract, app, 10u128.pow(24));
    }
    let alice = contract.internal_get_voter(&"alice.near".to_string());
    assert_eq!(alice.vote_positions.len(), 5);
}

#[test]
#[should_panic(expected = "Cannot exceed 5 voting positions.")]
fn one_position_more_than_configured_is_rejected() {
    let mut contract = contract_with_alice();
    for app in [
        "app1.near",
        "app2.near",
        "app3.near",
        "app4.near",
        "app5.near",
        "app6.near",
    ] {
        vote_on(&mut contract, app, 10u128.pow(24));
    }
}

#[test]
fn adding_votes_to_an_existing_position_works_at_the_limit() {
    let mut contract = contract_with_alice();
    for app in ["app1.near", "app2.near", "app3.near", "app4.near", "app5.near"] {
        vote_on(&mut contract, app, 10u128.pow(24));
    }
    vote_on(&mut contract, "app5.near", 10u128.pow(24));
    let alice = contract.internal_get_voter(&"alice.near".to_string());
    assert_eq!(alice.vote_positions.len(), 5);
}
