use crate::*;
use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
use near_sdk::{testing_env, PromiseResult, RuntimeFeesConfig, VMConfig};
use std::collections::HashMap;

const T0: u64 = 10 * 24 * 3600 * 1_000_000_000; // block timestamp in ns
const FIVE_NEAR: u128 = 5 * 10u128.pow(24);
const THRESHOLD: u128 = 1_000;

fn acct(name: &str) -> AccountId {
    name.parse().unwrap()
}

fn builder(predecessor: &str) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(acct("mpip.near"))
        .predecessor_account_id(acct(predecessor))
        .block_timestamp(T0);
    builder
}

fn new_contract() -> MpipContract {
    testing_env!(builder("admin.near").build());
    MpipContract::new(
        acct("admin.near"),
        acct("operator.near"),
        acct("meta.near"),
        acct("metavote.near"),
        U64(7 * 24 * 3600 * 1000),
        U128(THRESHOLD),
        U128(FIVE_NEAR),
        2500,
    )
}

/// runs the callback of create_proposal with the given meta-vote answer and attached deposit
fn create_callback(
    contract: &mut MpipContract,
    result: PromiseResult,
    deposit: u128,
) -> Option<MpipId> {
    testing_env!(
        builder("mpip.near").build(),
        VMConfig::test(),
        RuntimeFeesConfig::test(),
        HashMap::default(),
        vec![result]
    );
    contract.create_proposal_callback(
        acct("proposer.near"),
        U128(deposit),
        "title".to_string(),
        "short".to_string(),
        "body".to_string(),
        "data".to_string(),
        "extra".to_string(),
    )
}

fn voting_power(power: u128) -> PromiseResult {
    PromiseResult::Successful(format!("\"{}\"", power).into_bytes())
}

/// Debug text of the receipts created by the current call
fn receipts() -> String {
    format!("{:?}", get_created_receipts())
}

fn refund_of(receipts: &str, amount: u128) -> bool {
    receipts.contains("receiver_id: AccountId(\"proposer.near\")")
        && receipts.contains(&format!("Transfer {{ deposit: {} }}", amount))
}

#[test]
fn eligible_proposer_creates_the_proposal_and_keeps_only_the_storage_deposit() {
    let mut contract = new_contract();
    let id = create_callback(&mut contract, voting_power(THRESHOLD), FIVE_NEAR);
    assert_eq!(id, Some(0));
    assert_eq!(contract.proposals.len(), 1);
    assert!(!receipts().contains("Transfer"));
}

#[test]
fn overpayment_is_returned_when_the_proposal_is_created() {
    let mut contract = new_contract();
    let id = create_callback(&mut contract, voting_power(THRESHOLD), FIVE_NEAR + 10u128.pow(24));
    assert_eq!(id, Some(0));
    assert!(refund_of(&receipts(), 10u128.pow(24)));
}

#[test]
fn meta_vote_failure_refunds_the_whole_deposit() {
    let mut contract = new_contract();
    let id = create_callback(&mut contract, PromiseResult::Failed, FIVE_NEAR);
    assert_eq!(id, None);
    assert_eq!(contract.proposals.len(), 0);
    assert!(refund_of(&receipts(), FIVE_NEAR));
}

#[test]
fn unreadable_meta_vote_answer_refunds_the_whole_deposit() {
    let mut contract = new_contract();
    let id = create_callback(
        &mut contract,
        PromiseResult::Successful(b"not a number".to_vec()),
        FIVE_NEAR,
    );
    assert_eq!(id, None);
    assert!(refund_of(&receipts(), FIVE_NEAR));
}

#[test]
fn below_threshold_refunds_the_whole_deposit() {
    let mut contract = new_contract();
    let id = create_callback(&mut contract, voting_power(THRESHOLD - 1), FIVE_NEAR);
    assert_eq!(id, None);
    assert_eq!(contract.proposals.len(), 0);
    assert!(refund_of(&receipts(), FIVE_NEAR));
}
