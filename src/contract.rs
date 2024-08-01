use crate::{
    msg::ConfigResponse,
    storage::{Campaign, CampaignMeta, Link},
};
use cosmwasm_std::{Addr, Binary, Response, StdError, StdResult, Uint128, WasmMsg};
use cw_storage_plus::{Item, Map};
use sylvia::{
    contract, entry_points,
    types::{ExecCtx, InstantiateCtx, QueryCtx},
};

pub struct KickstarterContract {
    pub(crate) cw20_address: Item<Addr>,
    pub(crate) denom: Item<String>,
    pub(crate) campaign: Item<Campaign>,
    pub(crate) contributions: Map<Addr, Uint128>,
}

#[entry_points]
#[contract]
impl KickstarterContract {
    pub const fn new() -> Self {
        Self {
            cw20_address: Item::new("cw20_address"),
            denom: Item::new("denom"),
            campaign: Item::new("campaign"),
            contributions: Map::new("contributions"),
        }
    }

    #[sv::msg(instantiate)]
    pub fn instantiate(
        &self,
        context: InstantiateCtx,
        cw20_address: String,
        denom: String,
        campaign: CampaignMeta,
    ) -> StdResult<Response> {
        let cw20_address = context.deps.api.addr_validate(&cw20_address)?;

        let campaign = Campaign {
            name: campaign.name,
            description: campaign.description,
            end_time: campaign.end_time,
            links: campaign.links,
            tiers: campaign.tiers,
            creator: context.info.sender,
            minimum_contribution: campaign.minimum_contribution,
        };

        self.cw20_address
            .save(context.deps.storage, &cw20_address)?;
        self.denom.save(context.deps.storage, &denom)?;
        self.campaign.save(context.deps.storage, &campaign)?;

        Ok(Response::default()
            .add_attribute("action", "instantiate")
            .add_attribute("cw20_contract", cw20_address.to_string())
            .add_attribute("denom", denom)
            .add_attribute("campaign_name", campaign.name)
            .add_attribute("campaign_end_time", campaign.end_time.to_string())
            .add_attribute("campaign_creator", campaign.creator.to_string()))
    }

    #[sv::msg(exec)]
    pub fn update_campaign(
        &self,
        context: ExecCtx,
        description: String,
        links: Vec<Link>,
        minimum_contribution: Option<Uint128>,
    ) -> StdResult<Response> {
        let mut campaign = self.campaign.load(context.deps.storage)?;

        if campaign.creator != context.info.sender {
            return Err(StdError::generic_err("Unauthorized"));
        }

        campaign.description = description;
        campaign.links = links;
        campaign.minimum_contribution = minimum_contribution;

        self.campaign.save(context.deps.storage, &campaign)?;

        Ok(Response::default()
            .add_attribute("action", "update_campaign")
            .add_attribute("campaign_name", campaign.name)
            .add_attribute("campaign_end_time", campaign.end_time.to_string())
            .add_attribute("campaign_creator", campaign.creator.to_string()))
    }

    #[sv::msg(exec)]
    pub fn contribute(&self, context: ExecCtx) -> StdResult<Response> {
        let campaign = self.campaign.load(context.deps.storage)?;
        let cw20_address = self.cw20_address.load(context.deps.storage)?;

        if campaign.end_time < context.env.block.time {
            return Err(StdError::generic_err("Campaign has ended"));
        }

        let contribution = context.info.funds[0].clone();

        if contribution.denom != self.denom.load(context.deps.storage)? {
            return Err(StdError::generic_err("Invalid contribution denom"));
        }

        if let Some(minimum_contribution) = campaign.minimum_contribution {
            if contribution.amount < minimum_contribution {
                return Err(StdError::generic_err("Contribution too low"));
            }
        }

        let user_contribution = self
            .contributions
            .may_load(context.deps.storage, context.info.sender.clone())?;

        let new_contribution = match user_contribution {
            Some(contrib) => {
                self.contributions.update(
                    context.deps.storage,
                    context.info.sender.clone(),
                    |old| match old {
                        Some(prev) => Ok(prev + contribution.amount),
                        None => {
                            return Err(StdError::generic_err(
                                "Error occurred during contribution update",
                            ))
                        }
                    },
                )?;
                contrib + contribution.amount
            }
            None => {
                self.contributions.save(
                    context.deps.storage,
                    context.info.sender.clone(),
                    &contribution.amount,
                )?;
                contribution.amount
            }
        };

        let cw20_mint_msg = cw20::Cw20ExecuteMsg::Mint {
            recipient: context.info.sender.to_string(),
            amount: contribution.amount,
        };

        let cw20_msg = match serde_json::to_vec(&cw20_mint_msg) {
            Ok(vec) => Binary::from(vec),
            Err(e) => return Err(StdError::generic_err(format!("Serialization error: {}", e))),
        };

        let mint_cw20 = WasmMsg::Execute {
            contract_addr: cw20_address.to_string(),
            msg: cw20_msg,
            funds: vec![],
        };

        Ok(Response::default()
            .add_message(mint_cw20)
            .add_attribute("action", "contribute")
            .add_attribute("campaign", campaign.name)
            .add_attribute("contributor", context.info.sender.to_string())
            .add_attribute("contribution", new_contribution.to_string()))
    }

    pub fn refund(&self, context: InstantiateCtx) -> StdResult<Response> {
        let contribution = self
            .contributions
            .may_load(context.deps.storage, context.info.sender.clone())?;

        let contribution = match contribution {
            Some(contribution) => contribution,
            None => return Err(StdError::generic_err("No contribution found")),
        };

        let cw20_address = self.cw20_address.load(context.deps.storage)?;

        let cw20_burn_msg = cw20::Cw20ExecuteMsg::Burn {
            amount: contribution,
        };

        let cw20_msg = match serde_json::to_vec(&cw20_burn_msg) {
            Ok(vec) => Binary::from(vec),
            Err(e) => return Err(StdError::generic_err(format!("Serialization error: {}", e))),
        };

        let burn_cw20 = WasmMsg::Execute {
            contract_addr: cw20_address.to_string(),
            msg: cw20_msg,
            funds: vec![],
        };

        self.contributions
            .remove(context.deps.storage, context.info.sender.clone());

        Ok(Response::default()
            .add_message(burn_cw20)
            .add_attribute("action", "refund")
            .add_attribute("contributor", context.info.sender.to_string())
            .add_attribute("contribution", contribution.to_string()))
    }

    #[sv::msg(query)]
    pub fn info(&self, context: QueryCtx) -> StdResult<Campaign> {
        self.campaign.load(context.deps.storage)
    }

    #[sv::msg(query)]
    pub fn config(&self, context: QueryCtx) -> StdResult<ConfigResponse> {
        Ok(ConfigResponse {
            cw20_address: self.cw20_address.load(context.deps.storage)?,
            denom: self.denom.load(context.deps.storage)?,
        })
    }

    #[sv::msg(query)]
    pub fn contributions(&self, context: QueryCtx) -> StdResult<Vec<(Addr, Uint128)>> {
        self.contributions
            .range(
                context.deps.storage,
                None,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .collect()
    }
}
