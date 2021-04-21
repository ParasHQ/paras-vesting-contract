use near_sdk::{ext_contract, Gas};
use near_sdk::json_types::{U128};

pub const GAS_FOR_FT_TRANSFER: Gas = 10_000_000_000_000;
pub const ONE_MONTH: u64 = 2629746000000000; // 30.436875*24*60*60*10**9
//pub const NANO_SECONDS_PER_MONTH: u64 = 2628000_000_000_000;

#[ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_balance_of(&self, account_id: AccountId);
}

#[ext_contract(ext_self)]
pub trait Vesting {
    fn callback_revoke(
        &mut self,
    );
}
