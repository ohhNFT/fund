# Upload OhhFund

We are proposing to upload the code for the OhhFund contract to Stargaze Mainnet.

The source code is available at: https://github.com/OhhNFT/kickstarter

## Usecase

OhhFund allows projects, communities and any other organization to raise funds in a way similar to the Kickstarter platform (hence the contract's internal name).

## Starting a campaign

To start a new campaign, users can deploy their own contract using `Instantiate`, as follows:

```json
{
  "cw20_address": "stars1...",
  "denom": "ustars",
  "campaign": {
    "name": "Finish my Requiem before my untimely demise",
    "description": "Dies irae, dies illa. Solvet saeclum in favilla, teste David cum Sibylla. Quantus tremor est futurus, quando judex est venturus, cuncta stricte discussurus! [iykyk ;)]",
    "end_time": "14760478",
    "goal": "100000000000",
    "minimum_contribution": "1000000",
    "links": [
      {
        "name": "Twitter",
        "href": "https://twitter.com/josefleventon_"
      }
    ],
    "tiers": [
      {
        "name": "Small Fish",
        "description": "You should really contribute more",
        "required_contribution": "10000000"
      },
      {
        "name": "Big Fish",
        "description": "Thank you. You're a real one",
        "required_contribution": "100000000"
      },
      {
        "name": "Antonio Salieri",
        "description": "You're plotting against me aren't you?",
        "required_contribution": "100000000000"
      }
    ]
  }
}
```

For `denom`, we recommend USDC on Stargaze. On mainnet, the IBC denom is:

```
ibc/4A1C18CA7F50544760CF306189B810CE4C1CB156C7FC870143D401FE7280E591
```

Users will have to instantiate a CW20 contract beforehand, then update the minter address to the contract's address. To achieve this, users should first set themselves as the minter to avoid facing `Unauthorized` errors.

## Operating a campaign

While a campaign is running, users who contribute will receive an equal amount of CW20 tokens. These tokens can then be sent back at any time before the end of the campaign to receive a full refund.

## Ending a campaign

Once the campaign has ended, the creator can call `EndCampaign {}` to retrieve the funds locked in the contract.

### SHA256 checksum

```
26d6ec85cdb54895afbdca245550edff220f03f430c07db1edca255d5c1111d2  kickstarter.wasm
```
