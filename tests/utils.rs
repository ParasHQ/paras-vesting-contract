use paras_vesting_contract::ContractContract as VestingContract;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde_json::json;
use near_sdk_sim::{
    deploy, init_simulator, to_yocto, ContractAccount, UserAccount, DEFAULT_GAS, STORAGE_AMOUNT,
};

// Load in contract bytes at runtime
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    FT_WASM_BYTES => "res/fungible_token.wasm",
    VESTING_WASM_BYTES => "res/paras_vesting_contract.wasm",
}

pub const FT_ID: &str = "ft";
pub const VESTING_ID: &str = "vesting";
pub const ONE_MONTH: u64 = 2629746000000000; // 30.436875*24*60*60*10**9
pub const TWO_YEARS: u64 = ONE_MONTH * 12 * 2;
pub const JUNE_1_2021: u64 = 1622505600000000000; // Tuesday, June 1, 2021 12:00:00 AM GMT
pub const OCTOBER_1_2021: u64 = 1633046400000000000;
//const ONE_DAY:u64 = 86400000000000;
pub const SIX_MONTHS: u64 = ONE_MONTH * 6;
pub const ONE_MILLION_COIN: u128 = 1_000_000 * 10u128.pow(18);

/// PARAS to yoctoPARAS
pub fn ptoy(paras_amount: u128) -> u128 {
    paras_amount * 10u128.pow(18)
}

pub fn ytop(paras_amount: u128) -> u128 {
    paras_amount / 10u128.pow(18)
}

pub fn register_user(user: &near_sdk_sim::UserAccount) {
    user.call(
        FT_ID.to_string(),
        "storage_deposit",
        &json!({
            "account_id": user.valid_account_id()
        })
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS / 2,
        near_sdk::env::storage_byte_cost() * 125, // attached deposit
    )
    .assert_success();
}

pub fn init(is_one_month: bool) -> (UserAccount, UserAccount, ContractAccount<VestingContract>, UserAccount) {
    // Use `None` for default genesis configuration; more info below
    let root = init_simulator(None);

    let ft = root.deploy(
        &FT_WASM_BYTES,
        FT_ID.to_string(),
        STORAGE_AMOUNT, // attached deposit
    );
    ft.call(
        FT_ID.into(), 
        "new_paras_meta",
        &json!({
            "owner_id": root.valid_account_id(),
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS / 2,
        0,
    )
    .assert_success();

    let alice = root.create_user(
        "alice".to_string(),
        to_yocto("100") // initial balance
    );
    register_user(&alice);

    let vesting: ContractAccount<VestingContract>;

    if is_one_month {
        vesting = deploy!(
            contract: VestingContract,
            contract_id: VESTING_ID,
            bytes: &VESTING_WASM_BYTES,
            signer_account: root,
            init_method: new(
                root.valid_account_id(),
                alice.valid_account_id(),
                ft.valid_account_id(),
                U128::from(ONE_MILLION_COIN),
                (OCTOBER_1_2021 - ONE_MONTH).into(),
                TWO_YEARS.into(), // duration
                U64::from(0), // cliff
                true// revocable
                )
        );
    } else {
        vesting = deploy!(
            contract: VestingContract,
            contract_id: VESTING_ID,
            bytes: &VESTING_WASM_BYTES,
            signer_account: root,
            init_method: new(
                root.valid_account_id(),
                alice.valid_account_id(),
                ft.valid_account_id(),
                U128::from(ONE_MILLION_COIN),
                JUNE_1_2021.into(), // start
                TWO_YEARS.into(), // duration
                SIX_MONTHS.into(), // cliff
                true// revocable
                )
        );
    }

    register_user(&vesting.user_account);

    (root, ft, vesting, alice)
}