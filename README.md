# OhhNFT Fund Spec

Copyright © 2024 OhhNFT — All Rights Reserved

**fund** is a Stargaze smart contract that helps new projects and communities raise money in exchange for early perks, future tokens and virtually anything else.

## Campaigns

**fund** is meant to be deployed as a standalone contrarct for each project. When creating a new campaign, a project must set an end time no further than 1 year into the future.

The campaign's information is stored in the configuration of the contract as follows:

```rust
struct Campaign {
  pub name: String,
  pub description: Markdown,
  pub end_time: Timestamp,
  pub goal: Uint128,
  pub links: Vec<Link>,
  pub tiers: Vec<Tier>,
  pub creator: Addr,
  pub minimum_contribution: Option<Uint128>
}
```

## Reward Tiers

Campaigns can set different reward tiers that can be hit by contributors by contributing a certain amount to the project. These are defined as follows:

```rust
struct Tier {
  pub name: String,
  pub description: Markdown,
  pub required_contribution: Uint128
}
```

## Contributions

Contributions to a campaign are stored as follows:

```rust
struct Contribution {
  pub amount: Uint128
}
```

To contribute to a campaign, users can call `Contribute {}` with funds in USDC attached to the transaction. The key to the `Contribution` Item will be their address.

## Typing Particularities

### Links

For the purposes of providing accurate description and icons for strings, we define a `Link` type with a title, which can be set to values like "Discord" or "Twitter" to faciliate identifying links on frontends.

```rust
struct Link {
  pub name: String,
  pub href: String
}
```

### Markdown

To differentiate Markdown from generic strings, we use the `Markdown` type within the contract. This type is essentially just a string, so it is simply meant to indicate in TypeScript that the value should be formatted in Markdown for frontend display purposes.

```rust
type Markdown = String;
```
