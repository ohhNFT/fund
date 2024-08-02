#![cfg(test)]

use cosmwasm_std::{coin, coins, Addr, Binary, Coin, Empty, Timestamp, Uint128};
use cw20::{BalanceResponse, MinterResponse};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::ContributionResponse;

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

const INIT: &str = "init";

// Initial contract setup
fn setup_contracts() -> (App, Addr, Addr, Addr, Addr, Addr) {
    let init = Addr::unchecked(INIT);

    let init_funds = coins(2000, "ustars");

    let mut router = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &init, init_funds.clone())
            .unwrap();
    });

    let admin = router.api().addr_make("admin");
    let user = router.api().addr_make("user");
    let fee = router.api().addr_make("fee");

    router
        .send_tokens(init.clone(), user.clone(), &coins(1000, "ustars"))
        .unwrap();
    router
        .send_tokens(init, admin.clone(), &coins(1000, "ustars"))
        .unwrap();

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
            goal: Uint128::new(10000),
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
            &coins(1000, "ustars"),
            "KICKSTARTER",
            Some(admin.to_string()),
        )
        .unwrap();

    let msg = cw20::Cw20ExecuteMsg::UpdateMinter {
        new_minter: Some(kickstarter_addr.to_string()),
    };

    router
        .execute_contract(admin.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    let mut block = router.block_info();
    block.time = Timestamp::from_seconds(1);
    router.set_block(block);

    (router, cw20_addr, kickstarter_addr, admin, user, fee)
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

#[test]
fn try_contribute() {
    let (mut router, _, kickstarter_addr, _, user, _) = setup_contracts();

    // Contribute to the campaign
    let msg = crate::contract::sv::ExecMsg::Contribute {};
    router
        .execute_contract(
            user.clone(),
            kickstarter_addr.clone(),
            &msg,
            &[coin(100, "ustars".to_string())],
        )
        .unwrap();

    let campaign: crate::storage::Campaign = router
        .wrap()
        .query_wasm_smart(
            kickstarter_addr.clone(),
            &crate::contract::sv::QueryMsg::Info {},
        )
        .unwrap();
    assert_eq!(campaign.minimum_contribution.unwrap(), Uint128::new(100));

    // Ensure the user is now a contributor
    let contributors: Vec<ContributionResponse> = router
        .wrap()
        .query_wasm_smart(
            kickstarter_addr.clone(),
            &crate::contract::sv::QueryMsg::Contributions {},
        )
        .unwrap();
    assert_eq!(contributors.len(), 1);
    assert_eq!(contributors[0].contributor, user);

    // Ensure the user's contribution is recorded
    let contribution: Uint128 = router
        .wrap()
        .query_wasm_smart(
            kickstarter_addr.clone(),
            &crate::contract::sv::QueryMsg::Contribution {
                address: user.to_string(),
            },
        )
        .unwrap();
    assert_eq!(contribution, Uint128::new(100));
}

#[test]
fn try_contribute_below_minimum() {
    let (mut router, _, kickstarter_addr, _, user, _) = setup_contracts();

    // Contribute to the campaign
    let msg = crate::contract::sv::ExecMsg::Contribute {};
    let res = router.execute_contract(
        user.clone(),
        kickstarter_addr.clone(),
        &msg,
        &[coin(50, "ustars".to_string())],
    );

    assert!(res.is_err());
}

#[test]
fn try_refund() {
    let (mut router, cw20_addr, kickstarter_addr, _, user, _) = setup_contracts();

    // Contribute to the campaign
    let msg = crate::contract::sv::ExecMsg::Contribute {};
    router
        .execute_contract(
            user.clone(),
            kickstarter_addr.clone(),
            &msg,
            &[coin(100, "ustars".to_string())],
        )
        .unwrap();

    // Ensure the user's balance is updated
    let user_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: user.to_string(),
            },
        )
        .unwrap();
    assert_eq!(user_balance.balance, Uint128::new(100));

    // Refund the user
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: kickstarter_addr.to_string(),
        amount: Uint128::from(100u128),
        msg: Binary::new(b"{}".to_vec()),
    };
    router
        .execute_contract(user.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    // Ensure the user's balance is refunded
    let user_balance: Coin = router
        .wrap()
        .query_balance(user, "ustars".to_string())
        .unwrap();
    assert_eq!(user_balance.amount, Uint128::new(1000));

    // Ensure the user is no longer a contributor
    let contributors: Vec<ContributionResponse> = router
        .wrap()
        .query_wasm_smart(
            kickstarter_addr.clone(),
            &crate::contract::sv::QueryMsg::Contributions {},
        )
        .unwrap();
    assert_eq!(contributors.len(), 0);
}

#[test]
pub fn try_refund_without_contribution() {
    let (mut router, cw20_addr, kickstarter_addr, _, user, _) = setup_contracts();

    // Refund the user
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: kickstarter_addr.to_string(),
        amount: Uint128::from(100u128),
        msg: Binary::new(b"{}".to_vec()),
    };
    let res = router.execute_contract(user.clone(), cw20_addr.clone(), &msg, &[]);

    assert!(res.is_err());
}

#[test]
pub fn try_refund_too_many_tokens() {
    let (mut router, cw20_addr, kickstarter_addr, _, user, _) = setup_contracts();

    // Contribute to the campaign
    let msg = crate::contract::sv::ExecMsg::Contribute {};
    router
        .execute_contract(
            user.clone(),
            kickstarter_addr.clone(),
            &msg,
            &[coin(100, "ustars".to_string())],
        )
        .unwrap();

    // Ensure the user's balance is updated
    let user_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: user.to_string(),
            },
        )
        .unwrap();
    assert_eq!(user_balance.balance, Uint128::new(100));

    // Refund the user
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: kickstarter_addr.to_string(),
        amount: Uint128::from(101u128),
        msg: Binary::new(b"{}".to_vec()),
    };
    let res = router.execute_contract(user.clone(), cw20_addr.clone(), &msg, &[]);

    assert!(res.is_err());
}

#[test]
pub fn try_end_campaign() {
    let (mut router, cw20_addr, kickstarter_addr, admin, user, fee) = setup_contracts();

    // Contribute to the campaign
    let msg = crate::contract::sv::ExecMsg::Contribute {};
    router
        .execute_contract(
            user.clone(),
            kickstarter_addr.clone(),
            &msg,
            &[coin(100, "ustars".to_string())],
        )
        .unwrap();

    // Ensure the user's balance is updated
    let user_balance: BalanceResponse = router
        .wrap()
        .query_wasm_smart(
            cw20_addr.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: user.to_string(),
            },
        )
        .unwrap();
    assert_eq!(user_balance.balance, Uint128::new(100));

    // Push time to after campaign end
    add_block_time(&mut router, 86400);

    // End the campaign
    let msg = crate::contract::sv::ExecMsg::EndCampaign {};
    router
        .execute_contract(admin.clone(), kickstarter_addr.clone(), &msg, &[])
        .unwrap();

    // Ensure the admin has received the campaign balance
    let admin_balance: Coin = router
        .wrap()
        .query_balance(admin, "ustars".to_string())
        .unwrap();
    assert_eq!(admin_balance.amount, Uint128::new(95));

    // Ensure the fee account has received 5ustars
    let fee_balance: Coin = router
        .wrap()
        .query_balance(fee, "ustars".to_string())
        .unwrap();
    assert_eq!(fee_balance.amount, Uint128::new(1005));
}

#[test]
pub fn try_end_campaign_before_end() {
    let (mut router, _, kickstarter_addr, admin, _, _) = setup_contracts();

    // End the campaign
    let msg = crate::contract::sv::ExecMsg::EndCampaign {};
    let res = router.execute_contract(admin.clone(), kickstarter_addr.clone(), &msg, &[]);

    assert!(res.is_err());
}
