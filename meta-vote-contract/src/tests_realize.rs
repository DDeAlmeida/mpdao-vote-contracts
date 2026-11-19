use crate::*;
use near_sdk::json_types::U128;
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::testing_env;

fn acct(name: &str) -> AccountId {
    name.parse().unwrap()
}

const T0: u64 = 10 * 24 * 3600 * 1_000_000_000;
const ETH: &str = "aabbccddeeff00112233445566778899aabbccdd";

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

fn pseudo() -> String {
    format!("{ETH}.evmp.near")
}

fn mirror(contract: &mut MetaVoteContract, amount: u128) {
    testing_env!(context("operator.near").build());
    contract.operator_mirror_lps(ETH.to_string(), vec![(30, U128(amount))]);
}

fn link_delegate(contract: &mut MetaVoteContract, delegate: &str) {
    contract
        .evm_delegation_signatures
        .insert(ETH.to_string(), (delegate.to_string(), "sig".to_string()));
}

#[test]
#[should_panic(expected = "Only the operator can call this function.")]
fn non_operator_cannot_realize() {
    let mut contract = new_contract();
    mirror(&mut contract, ONE_MPDAO);
    link_delegate(&mut contract, "alice.near");
    testing_env!(context("bob.near").build());
    contract.realize_mirror(ETH.to_string());
}

#[test]
#[should_panic(expected = "no confirmed delegate")]
fn realize_requires_a_delegate() {
    let mut contract = new_contract();
    mirror(&mut contract, ONE_MPDAO);
    testing_env!(context("operator.near").build());
    contract.realize_mirror(ETH.to_string());
}

#[test]
fn realize_moves_locks_and_votes_once() {
    let mut contract = new_contract();
    mirror(&mut contract, ONE_MPDAO);
    let pseudo_id = pseudo();
    let info = contract.get_voter_info(&pseudo_id);
    let vp = u128::from(info.voting_power);
    testing_env!(context(&pseudo_id).build());
    contract.vote(U128(vp), "app.near".to_string(), "prop-1".to_string());

    link_delegate(&mut contract, "alice.near");
    let total_before = contract.total_voting_power;
    testing_env!(context("operator.near").build());
    contract.realize_mirror(ETH.to_string());

    assert!(contract.is_eth_mirror_realized(ETH.to_string()));
    assert_eq!(contract.total_voting_power, total_before);
    let cleared = contract.get_voter_info(&pseudo_id);
    assert!(cleared.locking_positions.is_empty());
    assert!(cleared.vote_positions.is_empty());

    let alice = contract.get_voter_info(&"alice.near".to_string());
    assert_eq!(alice.locking_positions.len(), 1);
    assert_eq!(alice.locking_positions[0].locking_period, 30);
    assert_eq!(alice.locking_positions[0].amount.0, ONE_MPDAO);
    assert_eq!(alice.vote_positions.len(), 1);
    assert_eq!(alice.vote_positions[0].voting_power.0, vp);

    testing_env!(context("operator.near").build());
    contract.operator_mirror_lps(ETH.to_string(), vec![(30, U128(ONE_MPDAO * 5))]);
    let still_clear = contract.get_voter_info(&pseudo_id);
    assert!(still_clear.locking_positions.is_empty());
    let alice_again = contract.get_voter_info(&"alice.near".to_string());
    assert_eq!(alice_again.locking_positions[0].amount.0, ONE_MPDAO);
}

#[test]
#[should_panic(expected = "eth address already realized")]
fn second_realize_reverts() {
    let mut contract = new_contract();
    mirror(&mut contract, ONE_MPDAO);
    link_delegate(&mut contract, "alice.near");
    testing_env!(context("operator.near").build());
    contract.realize_mirror(ETH.to_string());
    contract.realize_mirror(ETH.to_string());
}

// ---- gifts into EVM mirrors / realize with full delegate slots ----

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

fn gift(contract: &mut MetaVoteContract, to: &str, days: u16) {
    testing_env!(context("mpdao.near").build());
    contract.ft_on_transfer(
        acct("attacker.near"),
        U128(ONE_MPDAO),
        format!("[\"{to}\",{days}]"),
    );
}

#[test]
#[should_panic(expected = "Cannot lock into an EVM mirror account")]
fn cannot_gift_a_lock_into_an_evm_mirror() {
    let mut contract = new_contract();
    mirror(&mut contract, ONE_MPDAO);
    gift(&mut contract, &pseudo(), 60);
}

#[test]
#[should_panic(expected = "Cannot lock into an EVM mirror account")]
fn cannot_use_beneficiary_to_lock_into_an_evm_mirror() {
    let mut contract = new_contract();
    mirror(&mut contract, ONE_MPDAO);
    testing_env!(context("mpdao.near").build());
    contract.ft_on_transfer(
        acct("attacker.near"),
        U128(ONE_MPDAO),
        format!("{{\"days\":60,\"beneficiary\":\"{}\"}}", pseudo()),
    );
}

#[test]
fn realize_still_works_when_the_delegate_slots_were_filled_by_gifts() {
    let mut contract = new_contract();
    mirror(&mut contract, 5 * ONE_MPDAO); // mirror lock of 30 days
    link_delegate(&mut contract, "alice.near");
    // attacker fills all 5 slots of the delegate with tiny gifts at other periods
    for days in [60u16, 90, 120, 150, 180] {
        gift(&mut contract, "alice.near", days);
    }
    assert_eq!(contract.get_voter_info(&"alice.near".to_string()).locking_positions.len(), 5);

    testing_env!(context("operator.near").build());
    contract.realize_mirror(ETH.to_string());

    assert!(contract.is_eth_mirror_realized(ETH.to_string()));
    let alice = contract.get_voter_info(&"alice.near".to_string());
    assert_eq!(alice.locking_positions.len(), 6);
    assert!(alice
        .locking_positions
        .iter()
        .any(|p| p.locking_period == 30 && p.amount.0 == 5 * ONE_MPDAO));
}
