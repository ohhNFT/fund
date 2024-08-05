use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct ConfigResponse {
    pub cw20_address: Addr,
    pub denom: String,
}

#[cw_serde]
pub struct ContributionResponse {
    pub contributor: Addr,
    pub amount: Uint128,
}
