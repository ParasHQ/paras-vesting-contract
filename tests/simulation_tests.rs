use near_sdk::json_types::{U128, U64};
use near_sdk::{AccountId};
use near_sdk::serde_json::json;
use near_sdk::serde_json;
use near_sdk_sim::{call, transaction::ExecutionStatus, view, DEFAULT_GAS, UserAccount};
use chrono::{TimeZone, Utc};


// use utils::{init as init, register_user};
use crate::utils::{init, ptoy, SIX_MONTHS, TWO_YEARS, JUNE_1_2021};
mod utils;

#[test]
fn simulate_total_supply() {
    let (_, ft, _, _) = init();

    // let total_supply: U128 = view!(ft.ft_total_supply()).unwrap_json();
    let total_supply: U128 = ft.view(ft.account_id(), "ft_total_supply", b"").unwrap_json();

    assert_eq!(ptoy(100_000_000), total_supply.0);
}

#[test]
fn simulate_vesting_init() {
    let (_, ft, vesting, alice) = init();

    let recipient: AccountId = view!(vesting.recipient()).unwrap_json();
    println!("[VESTING] Recipient: {}", recipient.to_string());

    let token: AccountId = view!(vesting.token()).unwrap_json();
    println!("[VESTING] Token account: {}", token.to_string());

    let amount: U128 = view!(vesting.amount()).unwrap_json();
    println!("[VESTING] Total Amount: {}", serde_json::to_string(&amount).unwrap());

    let amount_claimed: U128 = view!(vesting.amount_claimed()).unwrap_json();
    println!("[VESTING] Total Amount Claimed: {}", serde_json::to_string(&amount_claimed).unwrap());

    let cliff: U64 = view!(vesting.cliff()).unwrap_json();
    let cliff: u64 = cliff.into();
    let cliff_dt = Utc.timestamp(cliff as i64 / 10i64.pow(9), 0);
    println!("[VESTING] Cliff ends at : {} , {}", cliff_dt.to_rfc2822(), cliff);


    let start: U64 = view!(vesting.start()).unwrap_json();
    let start: u64 = start.into();
    let start_dt = Utc.timestamp(start as i64 / 10i64.pow(9), 0);
    println!("[VESTING] Vesting starts at : {}, {}",start_dt.to_rfc2822(), start);

    let duration: U64 = view!(vesting.duration()).unwrap_json();
    let duration: u64 = duration.into();
    println!("[VESTING] Vesting duration : {}", duration);

    let vesting_end_dt = Utc.timestamp((cliff + duration) as i64 / 10i64.pow(9), 0);
    println!("[VESTING] Vesting ends at : {}, {}", vesting_end_dt.to_rfc2822(), cliff + duration);


    assert_eq!(recipient, alice.account_id);
    assert_eq!(token, ft.account_id);
    assert_eq!(amount, U128::from(ptoy(10_000_000)));
    assert_eq!(amount_claimed, U128::from(0));
    assert_eq!(cliff, JUNE_1_2021 + SIX_MONTHS);
    assert_eq!(start, JUNE_1_2021);
    assert_eq!(duration, TWO_YEARS);

}

fn send_amount_to_vesting(ft: &UserAccount, root: &UserAccount, vesting: &UserAccount) {
    // send amount from ft owner (root) to vesting contract
    root.call(
        ft.account_id(), 
        "ft_transfer",
        &json!({
            "receiver_id": vesting.account_id(),
            "amount": U128::from(ptoy(10_000_000))
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        1,
    )
    .assert_success();

    // check send amount success
    let vesting_balance: U128 = ft.view(
        ft.account_id(), 
        "ft_balance_of",
        &json!({
            "account_id": vesting.account_id()
        }).to_string().into_bytes(),
    ).unwrap_json();

    assert_eq!(vesting_balance, U128::from(ptoy(10_000_000)))
}
#[test]
fn simulate_claim_vested() {
    let (root, ft, vesting, alice) = init();
    send_amount_to_vesting(&ft, &root, &vesting.user_account);

    let outcome = call!(
        alice,
        vesting.claim_vested(),
        deposit = 0
    );

    // vesting is not yet begun
    assert_eq!(outcome.promise_errors().len(), 1);

    if let ExecutionStatus::Failure(execution_error) =
        &outcome.promise_errors().remove(0).unwrap().outcome().status
    {
        assert!(execution_error.to_string().contains("ERR_NO_VESTED_AMOUNT_ARE_DUE"));
    } else {
        unreachable!();
    }
}

#[test]
fn simulate_revoke() {
    let (root, ft, vesting, alice) = init();
    send_amount_to_vesting(&ft, &root, &vesting.user_account);

    let alice_balance_before: U128 = root.view(
        ft.account_id(),
        "ft_balance_of",
        &json!({
            "account_id": alice.account_id(),
        }).to_string().into_bytes()
    )
    .unwrap_json();

    let root_balance_before: U128 = root.view(
        ft.account_id(),
        "ft_balance_of",
        &json!({
            "account_id": root.account_id(),
        }).to_string().into_bytes()
    )
    .unwrap_json();

    let vesting_balance_before: U128 = root.view(
        ft.account_id(),
        "ft_balance_of",
        &json!({
            "account_id": vesting.account_id(),
        }).to_string().into_bytes()
    )
    .unwrap_json();

    let outcome = call!(
        root,
        vesting.revoke(root.valid_account_id()),
        deposit = 1
    );

    // Error because of amount = 0 for original recipient
    assert_eq!(outcome.promise_errors().len(), 1);

    // first outcome would be returned from vesting contract
    let first_outcome = outcome.promise_results().remove(1).unwrap();
    println!("[VESTING REVOKE] Tokens burnt: {} NEAR", first_outcome.tokens_burnt() as f64 / 10u128.pow(24) as f64);
    
    assert!(format!("{:?}", first_outcome.status())
        .contains(&ptoy(10_000_000).to_string()));

    // Expect error : The amount should be a positive number
    let second_outcome = outcome.promise_results().remove(2).unwrap();
    assert!(format!("{:?}", second_outcome.status())
        .contains("The amount should be a positive number"));

    // ft_transfer to root account
    let third_outcome = outcome.promise_results().remove(3).unwrap();
    assert!(format!("{:?}", third_outcome.logs()[0])
        .contains("Transfer 10000000000000000000000000000000 from vesting to root"));


    // verify if account is not active
    let start: U64 = view!(vesting.start()).unwrap_json();
    assert_eq!(start, U64::from(0));


    // verify recipient got all the amount
    let alice_balance_after: U128 = root.view(
        ft.account_id(),
        "ft_balance_of",
        &json!({
            "account_id": alice.account_id(),
        }).to_string().into_bytes()
    )
    .unwrap_json();

    let root_balance_after: U128 = root.view(
        ft.account_id(),
        "ft_balance_of",
        &json!({
            "account_id": root.account_id(),
        }).to_string().into_bytes()
    )
    .unwrap_json();

    let vesting_balance_after: U128 = root.view(
        ft.account_id(),
        "ft_balance_of",
        &json!({
            "account_id": vesting.account_id(),
        }).to_string().into_bytes()
    )
    .unwrap_json();

    let root_balance_before: u128 = root_balance_before.into();
    let vesting_balance_before: u128 = vesting_balance_before.into();

    assert_eq!(alice_balance_before, alice_balance_after);
    assert_eq!(root_balance_after, U128::from(root_balance_before + ptoy(10_000_000)));
    assert_eq!(vesting_balance_after, U128::from(vesting_balance_before - ptoy(10_000_000)));
 
}