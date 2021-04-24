use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen};
use near_sdk::json_types::{U128, ValidAccountId};
use near_sdk::{AccountId, Promise, PanicOnDefault, assert_one_yocto};
use near_contract_standards::upgrade::Ownable;

use crate::utils::{ext_fungible_token, ext_self, GAS_FOR_FT_TRANSFER, ONE_MONTH};
mod utils;

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner: AccountId,
    recipient: AccountId,
    token: AccountId,
    amount: u128,
    amount_claimed: u128,
    start: u64, 
    duration: u64,
    cliff: u64,
    revocable: bool,
    is_active: bool,
}

impl Ownable for Contract {
    fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    fn set_owner(&mut self, owner: AccountId) {
        self.assert_owner();
        self.owner = owner;
    }
}

/* 
    Implementation of vesting contract

    References:
    https://github.com/JoinColony/colonyToken/blob/master/contracts/Vesting.sol
    https://github.com/cpu-coin/CPUcoin/blob/master/contracts/ERC20Vestable.sol
    https://github.com/dreamteam-gg/smart-contracts/blob/master/contracts/vesting/DreamTokensVesting.sol
    https://modex.tech/developers/OpenZeppelinTeam/OpenZeppelin/src/master/contracts/drafts/TokenVesting.sol
*/
#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        owner: ValidAccountId,
        recipient : ValidAccountId,
        token: ValidAccountId,
        amount: U128,
        start: u64,
        duration: u64,
        cliff: u64,
        revocable: bool,
    ) -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_HAS_INITIALIZED");
        assert!(cliff < duration, "ERR_CLIFF_IS_HIGHER_THAN_DURATION");
        assert!(duration > 0, "ERR_DURATION_IS_LESS_THAN_ZERO");
        assert!((start.checked_add(duration.into()).expect("ERR_INTEGER_OVERFLOW")) > env::block_timestamp().into(), "ERR_START_AND_DURATION_IS_IN_THE_PAST");
        let this = Self {
            owner: owner.into(),
            recipient: recipient.into(),
            token: token.into(),
            amount: amount.into(),
            amount_claimed: 0,
            start: start,
            duration: duration,
            cliff: start.checked_add(cliff.into()).expect("ERR_INTEGER_OVERFLOW"),
            revocable: revocable,
            is_active: true,
        };
        this
    }

    pub fn recipient(&self) -> AccountId {
        self.recipient.clone()
    }

    pub fn amount(&self) -> U128 {
        self.amount.into()
    }

    pub fn token(&self) -> AccountId {
        self.token.clone()
    }

    pub fn amount_claimed(&self) -> U128 {
        self.amount_claimed.into()
    }

    pub fn cliff(&self) -> u64 {
        self.cliff
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn duration(&self) -> u64 {
        self.duration
    }

    pub fn revocable(&self) -> bool {
        self.revocable
    }

    pub fn claim_vested(&mut self) -> Promise {
        assert!(env::predecessor_account_id() == self.recipient(), "ERR_CALLER_NOT_RECIPIENT");
        assert!(self.is_active, "ERR_VESTING_CONTRACT_NOT_ACTIVE");
        let releasable = self.internal_releasable_amount();
        assert!(releasable > 0, "ERR_NO_VESTED_AMOUNT_ARE_DUE");

        self.amount_claimed = self.amount_claimed.checked_add(releasable).expect("ERR_INTEGER_OVERFLOW");

        ext_fungible_token::ft_transfer(
            self.recipient.clone(),
            releasable.into(),
            None,
            &self.token,
            1,
            GAS_FOR_FT_TRANSFER
        )
    }

    pub fn releasable_amount(&self) -> U128 {
        self.internal_releasable_amount().into()
    }

    #[private]
    pub fn internal_releasable_amount(&self) -> u128 {
        self.calculate_amount_vested().checked_sub(self.amount_claimed).expect("ERR_INTEGER_OVERFLOW")
    }

    pub fn calculate_amount_vested(&self) -> u128{
        if env::block_timestamp() < self.cliff {
            return 0;
        }

        let elapsed_time = env::block_timestamp().checked_sub(self.cliff).expect("ERR_INTEGER_OVERFLOW");

        if elapsed_time >= self.duration {
            let vested_amount = self.amount;
            return vested_amount;
        } else {
            let amount_per_months = self.amount / ( self.duration / ONE_MONTH ) as u128;
            let vested_amount = amount_per_months * ( elapsed_time / ONE_MONTH ) as u128;
            return vested_amount;
        }
    }

    #[payable]
    pub fn revoke(&mut self, recipient: ValidAccountId) -> u128 {
        self.assert_owner();
        assert_one_yocto();
        assert!(self.revocable == true, "ERR_GRANT_NOT_REVOCABLE");
        assert!(self.is_active, "ERR_VESTING_CONTRACT_NOT_ACTIVE");

        let releasable: u128 = self.internal_releasable_amount();
        let amount_not_vested: u128 = self.amount.checked_sub(self.amount_claimed).expect("Integer underflow").checked_sub(releasable).expect("Integer underflow");

        self.is_active = false;
        self.recipient = self.owner.clone();
        self.amount = 0;
        self.start = 0;
        self.duration = 0;
        self.cliff = 0;

        // transfer current amount_vested to original recipient
        ext_fungible_token::ft_transfer(
            self.recipient(),
            releasable.into(),
            None,
            &self.token,
            1,
            GAS_FOR_FT_TRANSFER
        );

        // transfer leftover to recipient specified
        ext_fungible_token::ft_transfer(
            recipient.into(),
            amount_not_vested.into(),
            None,
            &self.token,
            1,
            GAS_FOR_FT_TRANSFER
        );

        return amount_not_vested;
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env};

    const ONE_PARAS_TOKEN: U128 = U128(1_000_000_000_000_000_000_000_000);
    const TEN_PARAS_TOKEN: U128 = U128(10_000_000_000_000_000_000_000_000);
    const TEN_MILLION_PARAS_TOKEN: U128 = U128(10_000_000_000_000_000_000_000_000_000_000);
    const FIVE_HUNDRED_THOUSAND_PARAS_TOKEN: U128 = U128(500_000_000_000_000_000_000_000_000_000);
    const TOTAL_AMOUNT: U128 = FIVE_HUNDRED_THOUSAND_PARAS_TOKEN;

    // IN NANO SECONDS
    const ONE_MONTH:u64 = 2629746000000000; // 30.436875*24*60*60*10**9
    const TWO_YEARS: u64 = ONE_MONTH * 12 * 2;
    const JUNE_1_2021: u64 = 1622505600000000000; // Tuesday, June 1, 2021 12:00:00 AM GMT
    const ONE_DAY:u64 = 86400000000000;
    const SIX_MONTHS: u64 = ONE_MONTH * 6;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new(accounts(1).into(), accounts(3).into(), accounts(2).into(), TOTAL_AMOUNT, JUNE_1_2021, TWO_YEARS, SIX_MONTHS, true);
        (context, contract)
    }

    
    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new(accounts(1).into(), accounts(3).into(), accounts(2).into(), TOTAL_AMOUNT, JUNE_1_2021, TWO_YEARS, SIX_MONTHS, false);
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.get_owner(), accounts(1).to_string());
        assert_eq!(contract.recipient(), accounts(3).to_string());
        assert_eq!(contract.token(), accounts(2).to_string());
        assert_eq!(contract.amount(), TOTAL_AMOUNT);
        assert_eq!(contract.amount_claimed(), U128(0));
        assert_eq!(contract.start(), JUNE_1_2021);
        assert_eq!(contract.cliff(), JUNE_1_2021 + SIX_MONTHS);
        assert_eq!(contract.duration(), TWO_YEARS);
        assert_eq!(contract.revocable(), false);
        assert_eq!(contract.is_active, true);
    }

    #[test]
    fn test_calculate_amount_vested() {
        let (mut context, contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(1618109122863866400)
            .build()
        );
        let amount_vested = contract.calculate_amount_vested();
        assert_eq!(amount_vested, 0);

        // after start before cliff ONE DAY
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.start + ONE_DAY)
            .build()
        );
        let amount_vested = contract.calculate_amount_vested();
        assert_eq!(amount_vested, 0);

        // after start before cliff ONE MONTH
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.start + ONE_MONTH)
            .build()
        );
        let amount_vested = contract.calculate_amount_vested();
        assert_eq!(amount_vested, 0);

        // after cliff after ONE_DAY*29
        // month -> 0
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff + ONE_DAY*29)
            .build()
        );
        let amount_vested = contract.calculate_amount_vested();
        assert_eq!(amount_vested, 0);

        // after cliff after ONE MONTH
        // (FIVE_HUNDRED_THOUSAND_PARAS / (contract.duration / ONE_MONTH)) == 20833333333333333333333333333 == 20833.333333333332 PARAS/month
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff + ONE_MONTH)
            .build()
        );
        let amount_vested = contract.calculate_amount_vested();
        assert_eq!(amount_vested, (u128::from(TOTAL_AMOUNT) / (contract.duration as u128 / ONE_MONTH as u128)));

        // after cliff after ONE MONTH + 29 Days
        // (FIVE_HUNDRED_THOUSAND_PARAS / (contract.duration / ONE_MONTH)) == 20833333333333333333333333333 == 20833.333333333332 PARAS/month
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff + ONE_MONTH + ONE_DAY*29)
            .build()
        );
        let amount_vested = contract.calculate_amount_vested();
        assert_eq!(amount_vested, (u128::from(TOTAL_AMOUNT) / (contract.duration as u128 / ONE_MONTH as u128)));

        // after cliff after duration (vesting over)
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff + contract.duration + 1)
            .build()
        );
        let amount_vested = contract.calculate_amount_vested();
        assert_eq!(amount_vested, u128::from(TOTAL_AMOUNT));

    }

    #[test]
    fn test_claim_vested() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff)
            .build()
        );
        let releasable_amount = contract.internal_releasable_amount();
        assert_eq!(releasable_amount, 0);

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff + ONE_MONTH)
            .build()
        );
        let releasable_amount = contract.internal_releasable_amount();
        assert_eq!(releasable_amount, (u128::from(TOTAL_AMOUNT) / (contract.duration / ONE_MONTH) as u128));

        // claim
        contract.claim_vested();
        assert_eq!(contract.amount_claimed, (u128::from(TOTAL_AMOUNT) / (contract.duration / ONE_MONTH) as u128));

        // the next month
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff + ONE_MONTH*2)
            .build()
        );
        let releasable_amount = contract.internal_releasable_amount();
        assert_eq!(releasable_amount, (u128::from(TOTAL_AMOUNT) / (contract.duration / ONE_MONTH) as u128));

        // claim
        contract.claim_vested();
        assert_eq!(contract.amount_claimed, 2*(u128::from(TOTAL_AMOUNT) / (contract.duration / ONE_MONTH) as u128));

        // after vesting period over
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff + contract.duration + 1)
            .build()
        );

        let amount_vested = contract.calculate_amount_vested();
        assert_eq!(amount_vested, u128::from(TOTAL_AMOUNT));

        let releasable_amount = contract.internal_releasable_amount();
        assert_eq!(releasable_amount, u128::from(TOTAL_AMOUNT) - 2*(u128::from(TOTAL_AMOUNT) / (contract.duration / ONE_MONTH) as u128));

        contract.claim_vested();
        assert_eq!(contract.amount_claimed, u128::from(TOTAL_AMOUNT));

        // after claim everything
        let releasable_amount = contract.internal_releasable_amount();
        assert_eq!(releasable_amount, 0);
    }

    #[test]
    fn test_revoke() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff + ONE_MONTH)
            .build()
        );
        let releasable_amount = contract.internal_releasable_amount();
        assert_eq!(releasable_amount, (u128::from(TOTAL_AMOUNT) / (contract.duration / ONE_MONTH) as u128));

        // claim
        contract.claim_vested();
        assert_eq!(contract.amount_claimed, (u128::from(TOTAL_AMOUNT) / (contract.duration / ONE_MONTH) as u128));

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .block_timestamp(contract.cliff + ONE_MONTH)
            .attached_deposit(1)
            .build()
        );

        let current_amount_claimed = contract.amount_claimed();
        let releasable_amount = contract.internal_releasable_amount();
        // revoke
        let amount_not_vested = contract.revoke(accounts(1));
        assert_eq!(amount_not_vested, u128::from(TOTAL_AMOUNT) - u128::from(current_amount_claimed) - u128::from(releasable_amount));

        assert_eq!(contract.is_active, false);
        assert_eq!(contract.recipient(), accounts(1).to_string());
        assert_eq!(contract.amount, 0);
        assert_eq!(contract.start, 0);
        assert_eq!(contract.duration, 0);
        assert_eq!(contract.cliff, 0);

    }
    // NEGATIVE
    #[test]
    #[should_panic(expected = "ERR_NO_VESTED_AMOUNT_ARE_DUE")]
    fn test_invalid_claim_vested() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .block_timestamp(contract.cliff)
            .build()
        );
        contract.claim_vested();
    }
    #[test]
    #[should_panic(expected = "ERR_CALLER_NOT_RECIPIENT")]
    fn test_invalid_claim_vested_caller_not_recipient() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(4))
            .block_timestamp(contract.cliff+contract.duration)
            .build()
        );
        contract.claim_vested();
    }


}
