use crate::*;
use near_sdk::json_types::U128;
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::testing_env;

fn acct(name: &str) -> AccountId {
    name.parse().unwrap()
}

const T0: u64 = 10 * 24 * 3600 * 1_000_000_000;


fn context(predecessor: &str) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(acct("metavote.near"))
        .predecessor_account_id(acct(predecessor))
        .block_timestamp(T0);
    builder
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

fn voter_with_lock(contract: &mut MetaVoteContract, id: &str) -> u128 {
    let voter_id = id.to_string();
    let mut voter = contract.internal_get_voter(&voter_id);
    contract.internal_create_locking_position(&mut voter, 100 * ONE_MPDAO, 60);
    contract.voters.insert(&voter_id, &voter);
    contract.internal_get_voter(&voter_id).available_voting_power
}

#[test]
#[should_panic(expected = "A vote on a new object must be at least")]
fn dust_vote_on_a_new_object_is_rejected() {
    let mut contract = new_contract();
    voter_with_lock(&mut contract, "attacker.near");
    testing_env!(context("attacker.near").build());
    contract.vote(U128(1), "metastaking.app".to_string(), "junk-1".to_string());
}

#[test]
fn real_vote_and_top_ups_still_work() {
    let mut contract = new_contract();
    let power = voter_with_lock(&mut contract, "alice.near");
    testing_env!(context("alice.near").build());
    contract.vote(U128(ONE_MPDAO), "metastaking.app".to_string(), "val-1".to_string());
    // adding a few units to an object that already has a vote is fine
    contract.vote(U128(1), "metastaking.app".to_string(), "val-1".to_string());
    let votes = contract.get_votes_by_app("metastaking.app".to_string(), None, None);
    assert_eq!(votes, vec![("val-1".to_string(), U128(ONE_MPDAO + 1))]);
    assert!(power > ONE_MPDAO);
}

fn fresh_gas() {
    testing_env!(context("viewer.near").build());
}

fn fill_objects(contract: &mut MetaVoteContract, n: u64) {
    for i in 0..n {
        if i % 100 == 0 {
            fresh_gas(); // each testing_env! call starts with a new gas budget; storage is kept
        }
        contract.internal_increase_total_votes(
            1,
            &"metastaking.app".to_string(),
            &format!("obj-{i:05}"),
        );
    }
}

#[test]
fn views_are_paginated_and_default_to_a_bounded_page() {
    let mut contract = new_contract();
    fill_objects(&mut contract, 1250);
    let addr = "metastaking.app".to_string();
    fresh_gas();
    assert_eq!(contract.get_votes_by_app_count(addr.clone()), 1250);
    // default page is capped, not 1250 objects
    fresh_gas();
    assert_eq!(contract.get_votes_by_app(addr.clone(), None, None).len(), 500);
    fresh_gas();
    assert_eq!(contract.get_votes_by_contract(addr.clone(), None, None).len(), 500);
    // asking for more than the cap still returns at most the cap
    fresh_gas();
    assert_eq!(contract.get_votes_by_app(addr.clone(), Some(0), Some(100_000)).len(), 500);
    // paging walks the whole set without overlap
    fresh_gas();
    let p2 = contract.get_votes_by_app(addr.clone(), Some(500), Some(500));
    fresh_gas();
    let p3 = contract.get_votes_by_app(addr.clone(), Some(1000), Some(500));
    assert_eq!(p2.len(), 500);
    assert_eq!(p3.len(), 250);
    assert_eq!(p2[0].0, "obj-00500");
    assert_eq!(p3[249].0, "obj-01249");
    // out of range and unknown app
    fresh_gas();
    assert!(contract.get_votes_by_app(addr, Some(5000), Some(10)).is_empty());
    assert!(contract.get_votes_by_app("nope".to_string(), None, None).is_empty());
}
