use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct ConfigResponse {
    pub cw20_address: Addr,
    pub denom: String,
}
