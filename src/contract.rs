use crate::{
    msg::{ConfigResponse, ContributionResponse},
    storage::{Campaign, CampaignMeta, Link},
};
use cosmwasm_std::{
    coin, Addr, BankMsg, Binary, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
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

// Multitest
pub const FEE_ADDRESS: &str = "cosmwasm1hqxd4t5mxg4m523cl5uk9xtc9fxvdd9qenm8ln9me3she99yvqnqxhpk8e";

// Testnet
// pub const FEE_ADDRESS: &str = "stars1ggyrk0er22cpn8txw7gxyhvq2zn8dw598538jm";

// Mainnet
// pub const FEE_ADDRESS: &str = "stars1ggyrk0er22cpn8txw7gxyhvq2zn8dw598538jm";

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
            goal: campaign.goal,
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

        if context.info.funds.is_empty() {
            return Err(StdError::generic_err("No funds sent"));
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

    #[sv::msg(exec)]
    pub fn receive(
        &self,
        context: InstantiateCtx,
        sender: String,
        amount: Uint128,
    ) -> StdResult<Response> {
        let sender = context.deps.api.addr_validate(&sender)?;

        let contribution = self
            .contributions
            .may_load(context.deps.storage, sender.clone())?;

        let contribution = match contribution {
            Some(contribution) => contribution,
            None => return Err(StdError::generic_err("No contribution found")),
        };

        if amount > contribution {
            return Err(StdError::generic_err(
                "Amount sent is greater than contribution",
            ));
        }

        let cw20_address = self.cw20_address.load(context.deps.storage)?;

        let cw20_burn_msg = cw20::Cw20ExecuteMsg::Burn { amount };

        let cw20_msg = match serde_json::to_vec(&cw20_burn_msg) {
            Ok(vec) => Binary::from(vec),
            Err(e) => return Err(StdError::generic_err(format!("Serialization error: {}", e))),
        };

        let burn_cw20 = WasmMsg::Execute {
            contract_addr: cw20_address.to_string(),
            msg: cw20_msg,
            funds: vec![],
        };

        if amount < contribution {
            self.contributions
                .update(context.deps.storage, sender.clone(), |old| match old {
                    Some(prev) => Ok(prev - amount),
                    None => Err(StdError::generic_err(
                        "Error occurred during contribution update",
                    )),
                })?;
        } else {
            self.contributions
                .remove(context.deps.storage, sender.clone());
        }

        // Send tokens back to user
        let msg = BankMsg::Send {
            to_address: sender.to_string(),
            amount: vec![coin(amount.u128(), self.denom.load(context.deps.storage)?)],
        };

        let send_msg = SubMsg::new(msg);

        Ok(Response::default()
            .add_submessage(send_msg)
            .add_message(burn_cw20)
            .add_attribute("action", "refund")
            .add_attribute("contributor", context.info.sender.to_string())
            .add_attribute("contribution", contribution.to_string()))
    }

    #[sv::msg(exec)]
    pub fn end_campaign(&self, context: InstantiateCtx) -> StdResult<Response> {
        let campaign = self.campaign.load(context.deps.storage)?;
        let denom = self.denom.load(context.deps.storage)?;

        if campaign.creator != context.info.sender {
            return Err(StdError::generic_err("Unauthorized"));
        }

        if campaign.end_time >= context.env.block.time {
            return Err(StdError::generic_err("Campaign has not ended"));
        }

        let contract_address = context.env.contract.address.to_string();
        let contract_balance = context
            .deps
            .querier
            .query_balance(&contract_address, denom.clone())
            .map_err(|error| error)?;

        let fee_amount = contract_balance.amount.u128() / 20;
        let fee_msg = BankMsg::Send {
            to_address: FEE_ADDRESS.to_string(),
            amount: vec![coin(fee_amount, denom.clone())],
        };
        let fee_send_msg = SubMsg::new(fee_msg);

        let msg = BankMsg::Send {
            to_address: context.info.sender.to_string(),
            amount: vec![coin(
                contract_balance.amount.u128() - fee_amount,
                denom.clone(),
            )],
        };
        let send_msg = SubMsg::new(msg);

        self.contributions.clear(context.deps.storage);

        Ok(Response::default()
            .add_submessage(send_msg)
            .add_submessage(fee_send_msg)
            .add_attribute("action", "end_campaign")
            .add_attribute("campaign", campaign.name)
            .add_attribute("total_contributions", contract_balance.amount.to_string()))
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
    pub fn contributions(&self, context: QueryCtx) -> StdResult<Vec<ContributionResponse>> {
        self.contributions
            .range(
                context.deps.storage,
                None,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .map(|item| {
                item.map(|(contributor, amount)| ContributionResponse {
                    contributor,
                    amount,
                })
            })
            .collect()
    }

    #[sv::msg(query)]
    pub fn contribution(&self, context: QueryCtx, address: String) -> StdResult<Uint128> {
        Ok(self
            .contributions
            .load(
                context.deps.storage,
                context.deps.api.addr_validate(&address)?,
            )
            .unwrap_or(Uint128::zero()))
    }
}
