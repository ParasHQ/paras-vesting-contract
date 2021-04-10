use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen};
use near_sdk::json_types::{U128, ValidAccountId};
use near_sdk::{AccountId, Balance, Promise, PanicOnDefault, assert_one_yocto, log};
use near_contract_standards::upgrade::Ownable;

use crate::utils::{ext_fungible_token, ext_self, GAS_FOR_FT_TRANSFER};
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

    pub fn amount(&self) -> u128 {
        self.amount
    }

    pub fn token(&self) -> AccountId {
        self.token.clone()
    }

    pub fn amount_claimed(&self) -> u128 {
        self.amount_claimed
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
        let releasable = self.releasable_amount();
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

    pub fn releasable_amount(&self) -> u128 {
        self.calculate_amount_vested().checked_sub(self.amount_claimed()).expect("ERR_INTEGER_OVERFLOW")
    }

    pub fn calculate_amount_vested(&self) -> u128{
        if env::block_timestamp() < self.cliff {
            return 0;
        }

        let elapsed_time = env::block_timestamp() - self.cliff;

        if elapsed_time >= self.duration {
            let vested_amount = self.amount.checked_sub(self.amount_claimed).expect("ERR_INTEGER_OVERFLOW");
            return vested_amount;
        } else {
            let vested_amount = self.amount * (elapsed_time as u128 / self.duration as u128);
            return vested_amount;
        }
    }

    // Design Choice : 
    // 1. Revoke + send amount not vested (all amount) to recipient
    // 2. Revoke and sweep_grant -> ownerOnly
    // 3. no revoke function

    pub fn sweep_grant(&mut self, amount: Balance) -> Promise {
        self.assert_owner();
        assert!(true, "ERR_FUNCTION_DISABLED");
        assert!(!self.is_active, "ERR_VESTING_CONTRACT_STILL_ACTIVE");
        ext_fungible_token::ft_transfer(
            self.owner.clone().into(),
            amount.into(),
            None,
            &self.token,
            1,
            GAS_FOR_FT_TRANSFER
        )
    }

    /*
    pub fn revoke(&mut self) {
        self.assert_owner();
        assert_one_yocto();
        assert!(self.revocable == true, "Grant non revocable");
        self.is_active = false;
        self.recipient = self.owner.clone();
        self.amount = 0;
        self.amount_claimed = 0;
        self.start = 0;
        self.duration = 0;
        self.cliff = 0;
    }
    */

    pub fn revoke(&mut self, recipient: ValidAccountId) -> Promise {
        self.assert_owner();
        assert_one_yocto();
        assert!(true, "ERR_FUNCTION_DISABLED");
        assert!(self.revocable == true, "ERR_GRANT_NOT_REVOCABLE");
        assert!(self.is_active, "ERR_VESTING_CONTRACT_NOT_ACTIVE");

        let amount_vested: u128 = self.calculate_amount_vested();
        let amount_not_vested: u128 = self.amount.checked_sub(self.amount_claimed).expect("Integer underflow").checked_sub(amount_vested).expect("Integer underflow");

        ext_fungible_token::ft_transfer(
            recipient.into(),
            amount_not_vested.into(),
            None,
            &self.token,
            1,
            GAS_FOR_FT_TRANSFER
        ).then(
        ext_self::callback_revoke(
            &env::current_account_id(),
            0,
            env::prepaid_gas() - GAS_FOR_FT_TRANSFER
        )
        )
    }


    #[private]
    pub fn callback_revoke(&mut self) {
        // ft_transfer returns void, there is no way to make sure ft_transfer successful
        self.is_active = false;
        self.recipient = self.owner.clone();
        self.amount = 0;
        self.amount_claimed = 0;
        self.start = 0;
        self.duration = 0;
        self.cliff = 0;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "carol_near".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }
/* 
    #[test]
    fn set_get_message() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = StatusMessage::default();
        contract.set_status("hello".to_string());
        assert_eq!(
            "hello".to_string(),
            contract.get_status("bob_near".to_string()).unwrap()
        );
    }

    #[test]
    fn get_nonexistent_message() {
        let context = get_context(vec![], true);
        testing_env!(context);
        let contract = StatusMessage::default();
        assert_eq!(None, contract.get_status("francis.near".to_string()));
    } */
}
