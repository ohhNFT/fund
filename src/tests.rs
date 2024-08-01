#![cfg(test)]

use cosmwasm_std::{coins, Addr, Empty, Timestamp, Uint128};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn contract_kickstarter() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::entry_points::execute,
        crate::contract::entry_points::instantiate,
        crate::contract::entry_points::query,
    );
    Box::new(contract)
}

const CW20: &str = "contract0";
const KICKSTARTER: &str = "contract1";

const ADMIN: &str = "admin";
const USER: &str = "user";

// Initial contract setup
fn setup_contracts() -> App {
    let admin = Addr::unchecked(ADMIN);

    let init_funds = coins(2000, "ustars");

    let mut router = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &admin, init_funds)
            .unwrap();
    });

    // Set up CW20 contract
    let cw20_id = router.store_code(contract_cw20());
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("My Campaign Token"),
        symbol: String::from("MCT"),
        decimals: 6,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: admin.to_string(),
            cap: None,
        }),
        marketing: None,
    };
    let cw20_addr = router
        .instantiate_contract(cw20_id, admin.clone(), &msg, &[], "MCT_CW20", None)
        .unwrap();

    // Set up Kickstarter contract
    let kickstarter_id = router.store_code(contract_kickstarter());
    let msg = crate::contract::sv::InstantiateMsg {
        cw20_address: cw20_addr.to_string(),
        denom: "ustars".to_string(),
        campaign: crate::storage::CampaignMeta {
            name: "My Campaign".to_string(),
            description: "My Campaign Description".to_string(),
            end_time: Timestamp::from_seconds(86400),
            links: vec![],
            tiers: vec![],
            minimum_contribution: Some(Uint128::new(100)),
        },
    };

    let kickstarter_addr = router
        .instantiate_contract(
            kickstarter_id,
            admin.clone(),
            &msg,
            &[],
            "KICKSTARTER",
            None,
        )
        .unwrap();

    let msg = cw20::Cw20ExecuteMsg::UpdateMinter {
        new_minter: Some(kickstarter_addr.to_string()),
    };

    router
        .execute_contract(admin, cw20_addr, &msg, &[])
        .unwrap();

    let mut block = router.block_info();
    block.time = Timestamp::from_seconds(1);
    router.set_block(block);

    router
}

// Update block time
fn add_block_time(router: &mut App, seconds: u64) {
    let mut block = router.block_info();
    block.time = block.time.plus_seconds(seconds);
    router.set_block(block);
}

#[test]
fn proper_initialization() {
    setup_contracts();
}
