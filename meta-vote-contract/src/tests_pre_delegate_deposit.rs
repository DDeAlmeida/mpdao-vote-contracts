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

use near_sdk::test_utils::get_created_receipts;

const EVM: &str = "0xabc";

fn deposit() -> u128 {
    crate::evm_delegate::PRE_DELEGATE_DEPOSIT
}

fn pre_delegate(contract: &mut MetaVoteContract, account: &str, evm: &str) {
    testing_env!(context(account).attached_deposit(deposit()).build());
    contract.pre_delegate_evm_address(evm.to_string(), "signature".to_string());
}

/// Debug text of the receipts created by the current call
fn receipts() -> String {
    format!("{:?}", get_created_receipts())
}

fn is_refund_to(receipts: &str, account: &str) -> bool {
    receipts.contains(&format!("receiver_id: AccountId(\"{}\")", account))
        && receipts.contains(&format!("Transfer {{ deposit: {} }}", deposit()))
}

#[test]
#[should_panic(expected = "refundable storage deposit")]
fn pre_delegation_needs_the_storage_deposit() {
    let mut contract = new_contract();
    testing_env!(context("alice.near").attached_deposit(1).build());
    contract.pre_delegate_evm_address(EVM.to_string(), "signature".to_string());
}

#[test]
fn pre_delegation_with_the_deposit_is_stored() {
    let mut contract = new_contract();
    pre_delegate(&mut contract, "alice.near", EVM);
    let stored = contract.get_pre_delegate_evm_address(EVM.to_string()).unwrap();
    assert_eq!(stored.0, "alice.near");
}

#[test]
fn operator_removal_refunds_the_payer() {
    let mut contract = new_contract();
    pre_delegate(&mut contract, "alice.near", EVM);
    testing_env!(context("operator.near").attached_deposit(1).build());
    contract.operator_remove_pre_delegate_evm_address(EVM.to_string());
    assert!(is_refund_to(&receipts(), "alice.near"));
    assert!(contract.get_pre_delegate_evm_address(EVM.to_string()).is_none());
}

#[test]
fn confirming_a_delegation_refunds_the_payer() {
    let mut contract = new_contract();
    pre_delegate(&mut contract, "alice.near", EVM);
    testing_env!(context("operator.near").attached_deposit(1).build());
    contract.operator_confirm_delegated_evm_address(EVM.to_string());
    assert!(is_refund_to(&receipts(), "alice.near"));
    let delegated = contract.evm_delegates.get(&"alice.near".to_string()).unwrap();
    assert_eq!(delegated, vec![EVM.to_string()]);
}

#[test]
fn an_overwritten_pre_delegation_refunds_the_previous_payer() {
    let mut contract = new_contract();
    pre_delegate(&mut contract, "alice.near", EVM);
    pre_delegate(&mut contract, "bob.near", EVM);
    assert!(is_refund_to(&receipts(), "alice.near"));
    let stored = contract.get_pre_delegate_evm_address(EVM.to_string()).unwrap();
    assert_eq!(stored.0, "bob.near");
}

#[test]
fn entries_created_before_the_deposit_are_never_refunded() {
    let mut contract = new_contract();
    contract.evm_pre_delegation.insert(
        "0xold".to_string(),
        ("carol.near".to_string(), "signature".to_string()),
    );
    testing_env!(context("operator.near").attached_deposit(1).build());
    contract.operator_remove_pre_delegate_evm_address("0xold".to_string());
    assert!(!receipts().contains("carol.near"));
    assert!(contract.get_pre_delegate_evm_address("0xold".to_string()).is_none());
}
