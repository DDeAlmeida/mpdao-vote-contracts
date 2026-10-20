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

const EVM: &str = "0xabc";

fn pseudo() -> String {
    crate::utils::pseudo_near_address(&EVM.to_string())
}

/// the evm user has 100 mpDAO of unlocked rewards under the pseudo account, delegate.near is the delegate
fn contract_with_unlocked_rewards() -> MetaVoteContract {
    let mut contract = new_contract();
    contract
        .claimable_unlocked_mpdao
        .insert(&pseudo(), &(100 * ONE_MPDAO));
    contract.total_unclaimed_unlocked_mpdao = 100 * ONE_MPDAO;
    contract
        .evm_delegates
        .insert(&"delegate.near".to_string(), &vec![EVM.to_string()]);
    contract
}

#[test]
fn delegated_unbond_bonds_to_the_delegate_not_to_the_pseudo_account() {
    let mut contract = contract_with_unlocked_rewards();
    testing_env!(context("delegate.near").attached_deposit(1).build());
    contract.delegated_claim_unlocked_mpdao(EVM.to_string(), U128(40 * ONE_MPDAO), Some(180));

    let delegate = contract.internal_get_voter(&"delegate.near".to_string());
    assert_eq!(delegate.locking_positions.len(), 1);
    assert_eq!(delegate.get_position(0).amount, 40 * ONE_MPDAO);
    assert_eq!(delegate.get_position(0).locking_period, 180);
    // nothing was created under the pseudo account, nobody could ever unlock it
    assert!(contract.voters.get(&pseudo()).is_none());
    // the claim was taken from the pseudo account
    assert_eq!(
        contract.claimable_unlocked_mpdao.get(&pseudo()).unwrap(),
        60 * ONE_MPDAO
    );
    assert_eq!(contract.total_unclaimed_unlocked_mpdao, 60 * ONE_MPDAO);
}

#[test]
fn own_unbond_claim_still_bonds_to_the_caller() {
    let mut contract = new_contract();
    contract
        .claimable_unlocked_mpdao
        .insert(&"bob.near".to_string(), &(100 * ONE_MPDAO));
    contract.total_unclaimed_unlocked_mpdao = 100 * ONE_MPDAO;

    testing_env!(context("bob.near").build());
    contract.claim_unlocked_mpdao(U128(40 * ONE_MPDAO), Some(180));

    let bob = contract.internal_get_voter(&"bob.near".to_string());
    assert_eq!(bob.locking_positions.len(), 1);
    assert_eq!(bob.get_position(0).amount, 40 * ONE_MPDAO);
}

#[test]
#[should_panic(expected = "is not delegated to")]
fn only_the_delegate_can_claim() {
    let mut contract = contract_with_unlocked_rewards();
    testing_env!(context("attacker.near").attached_deposit(1).build());
    contract.delegated_claim_unlocked_mpdao(EVM.to_string(), U128(40 * ONE_MPDAO), Some(180));
}
