use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};

#[cw_serde]
pub struct Link {
    pub name: String,
    pub href: String,
}

type Markdown = String;

#[cw_serde]
pub struct Tier {
    pub name: String,
    pub description: Markdown,
    pub required_contribution: Uint128,
}

#[cw_serde]
pub struct Campaign {
    pub name: String,
    pub description: Markdown,
    pub end_time: Timestamp,
    pub goal: Uint128,
    pub links: Vec<Link>,
    pub tiers: Vec<Tier>,
    pub creator: Addr,
    pub minimum_contribution: Option<Uint128>,
}

#[cw_serde]
pub struct CampaignMeta {
    pub name: String,
    pub description: Markdown,
    pub end_time: Timestamp,
    pub goal: Uint128,
    pub links: Vec<Link>,
    pub tiers: Vec<Tier>,
    pub minimum_contribution: Option<Uint128>,
}
