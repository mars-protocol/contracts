/// Modified based on the original file from the cw-it repository:
/// https://github.com/apollodao/cw-it/blob/master/src/multi_test/modules/token_factory.rs
/// It uses different functions input arguments types and newer versions of the dependencies.
use std::{fmt::Debug, str::FromStr};

use anyhow::{bail, Result as AnyResult};
use cosmwasm_std::{
    from_json, testing::MockApi, Addr, Api, BankMsg, BankQuery, Binary, BlockInfo, Coin,
    CustomQuery, Empty, Event, GovMsg, IbcMsg, IbcQuery, MemoryStorage, Querier, QueryRequest,
    Storage, SupplyResponse, Uint128,
};
use cw_multi_test::{
    App, AppResponse, BankKeeper, BankSudo, CosmosRouter, DistributionKeeper, FailingModule,
    StakeKeeper, Stargate, WasmKeeper,
};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgBurnResponse, MsgCreateDenom, MsgCreateDenomResponse, MsgMint, MsgMintResponse,
};
use regex::Regex;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

pub type CustomApp = App<
    BankKeeper,
    MockApi,
    MemoryStorage,
    FailingModule<Empty, Empty, Empty>,
    WasmKeeper<Empty, Empty>,
    StakeKeeper,
    DistributionKeeper,
    FailingModule<IbcMsg, IbcQuery, Empty>,
    FailingModule<GovMsg, Empty, Empty>,
    TokenFactory,
>;

impl Stargate for TokenFactory {
    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        type_url: String,
        msg: Binary,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        match type_url.as_str() {
            MsgCreateDenom::TYPE_URL => self.create_denom(api, storage, router, block, sender, msg),
            MsgMint::TYPE_URL => self.mint(api, storage, router, block, sender, msg),
            MsgBurn::TYPE_URL => self.burn(api, storage, router, block, sender, msg),
            _ => bail!("Unknown message type {}", type_url),
        }
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _path: String,
        _data: Binary,
    ) -> AnyResult<Binary> {
        bail!("stargate query called")
    }
}

#[derive(Clone)]
pub struct TokenFactory {
    pub module_denom_prefix: String,
    pub max_subdenom_len: usize,
    pub max_hrp_len: usize,
    pub max_creator_len: usize,
    pub denom_creation_fee: String,
}

impl TokenFactory {
    /// Creates a new TokenFactory instance with the given parameters.
    pub fn new(
        prefix: &str,
        max_subdenom_len: usize,
        max_hrp_len: usize,
        max_creator_len: usize,
        denom_creation_fee: &str,
    ) -> Self {
        Self {
            module_denom_prefix: prefix.to_string(),
            max_subdenom_len,
            max_hrp_len,
            max_creator_len,
            denom_creation_fee: denom_creation_fee.to_string(),
        }
    }
}

impl Default for TokenFactory {
    fn default() -> Self {
        Self::new("factory", 32, 16, 59 + 16, "10000000untrn")
    }
}

impl TokenFactory {
    fn create_denom<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: Binary,
    ) -> anyhow::Result<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        let msg: MsgCreateDenom = msg.try_into()?;

        // Validate subdenom length
        if msg.subdenom.len() > self.max_subdenom_len {
            bail!("Subdenom length is too long, max length is {}", self.max_subdenom_len);
        }
        // Validate creator length
        if msg.sender.len() > self.max_creator_len {
            bail!("Creator length is too long, max length is {}", self.max_creator_len);
        }
        // Validate creator address not contains '/'
        if msg.sender.contains('/') {
            bail!("Invalid creator address, creator address cannot contains '/'");
        }
        // Validate sender is the creator
        if msg.sender != sender {
            bail!("Invalid creator address, creator address must be the same as the sender");
        }

        let denom = format!("{}/{}/{}", self.module_denom_prefix, msg.sender, msg.subdenom);

        println!("denom: {}", denom);

        // Query supply of denom
        let request = QueryRequest::Bank(BankQuery::Supply {
            denom: denom.clone(),
        });
        let raw = router.query(api, storage, block, request)?;
        let supply: SupplyResponse = from_json(raw)?;
        println!("supply: {:?}", supply);
        println!("supply.amount.amount.is_zero: {:?}", supply.amount.amount.is_zero());
        if !supply.amount.amount.is_zero() {
            println!("bailing");
            bail!("Subdenom already exists");
        }

        // Charge denom creation fee
        let fee = coin_from_sdk_string(&self.denom_creation_fee)?;
        let fee_msg = BankMsg::Burn {
            amount: vec![fee],
        };
        router.execute(api, storage, block, sender, fee_msg.into())?;

        let create_denom_response = MsgCreateDenomResponse {
            new_token_denom: denom.clone(),
        };

        let mut res = AppResponse::default();
        res.events.push(
            Event::new("create_denom")
                .add_attribute("creator", msg.sender)
                .add_attribute("new_token_denom", denom),
        );
        res.data = Some(create_denom_response.into());

        Ok(res)
    }

    pub fn mint<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: Binary,
    ) -> anyhow::Result<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        let msg: MsgMint = msg.try_into()?;

        let denom = msg.amount.clone().unwrap().denom;
        println!("Mint denom: {}", denom);

        // Validate sender
        let parts = denom.split('/').collect::<Vec<_>>();
        if parts[1] != sender {
            bail!("Unauthorized mint. Not the creator of the denom.");
        }
        if sender != msg.sender {
            bail!("Invalid sender. Sender in msg must be same as sender of transaction.");
        }

        // Validate denom
        if parts.len() != 3 && parts[0] != self.module_denom_prefix {
            bail!("Invalid denom");
        }

        let amount = Uint128::from_str(&msg.amount.unwrap().amount)?;
        if amount.is_zero() {
            bail!("Invalid zero amount");
        }

        // Mint through BankKeeper sudo method
        let mint_msg = BankSudo::Mint {
            to_address: msg.mint_to_address.clone(),
            amount: vec![Coin {
                denom: denom.clone(),
                amount,
            }],
        };
        router.sudo(api, storage, block, mint_msg.into())?;

        let mut res = AppResponse::default();
        let data = MsgMintResponse {};
        res.data = Some(data.into());
        res.events.push(
            Event::new("tf_mint")
                .add_attribute("mint_to_address", msg.mint_to_address.to_string())
                .add_attribute("amount", amount.to_string()),
        );
        Ok(res)
    }

    pub fn burn<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: Binary,
    ) -> anyhow::Result<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        let msg: MsgBurn = msg.try_into()?;

        // Validate sender
        let denom = msg.amount.clone().unwrap().denom;
        let parts = denom.split('/').collect::<Vec<_>>();
        if parts[1] != sender {
            bail!("Unauthorized burn. Not the creator of the denom.");
        }
        if sender != msg.sender {
            bail!("Invalid sender. Sender in msg must be same as sender of transaction.");
        }

        // Validate denom
        if parts.len() != 3 && parts[0] != self.module_denom_prefix {
            bail!("Invalid denom");
        }

        let amount = Uint128::from_str(&msg.amount.unwrap().amount)?;
        if amount.is_zero() {
            bail!("Invalid zero amount");
        }

        // Burn through BankKeeper
        let burn_msg = BankMsg::Burn {
            amount: vec![Coin {
                denom: denom.clone(),
                amount,
            }],
        };
        router.execute(api, storage, block, sender.clone(), burn_msg.into())?;

        let mut res = AppResponse::default();
        let data = MsgBurnResponse {};
        res.data = Some(data.into());

        res.events.push(
            Event::new("tf_burn")
                .add_attribute("burn_from_address", sender.to_string())
                .add_attribute("amount", amount.to_string()),
        );

        Ok(res)
    }
}

fn coin_from_sdk_string(sdk_string: &str) -> anyhow::Result<Coin> {
    let denom_re = Regex::new(r"^[0-9]+[a-z]+$")?;
    let ibc_re = Regex::new(r"^[0-9]+(ibc|IBC)/[0-9A-F]{64}$")?;
    let factory_re = Regex::new(r"^[0-9]+factory/[0-9a-z]+/[0-9a-zA-Z]+$")?;

    if !(denom_re.is_match(sdk_string)
        || ibc_re.is_match(sdk_string)
        || factory_re.is_match(sdk_string))
    {
        bail!("Invalid sdk string");
    }

    // Parse amount
    let re = Regex::new(r"[0-9]+")?;
    let amount = re.find(sdk_string).unwrap().as_str();
    let amount = Uint128::from_str(amount)?;

    // The denom is the rest of the string
    let denom = sdk_string[amount.to_string().len()..].to_string();

    Ok(Coin {
        denom,
        amount,
    })
}
