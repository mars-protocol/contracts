use cosmwasm_schema::cw_serde;
use cw_vault_standard::{VaultStandardExecuteMsg, VaultStandardQueryMsg};

pub type ExecuteMsg = VaultStandardExecuteMsg<ExtensionExecuteMsg>;

pub type QueryMsg = VaultStandardQueryMsg<ExtensionQueryMsg>;

#[cw_serde]
pub struct InstantiateMsg {
    /// The base token denom that will be used for the native vault token, e.g. uusdc.
    pub base_token: String,
    /// The subdenom that will be used for the native vault token, e.g.
    /// the denom of the vault token will be:
    /// "factory/{vault_contract}/{vault_token_subdenom}".
    pub vault_token_subdenom: String,

    /// Optional metadata
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub description: Option<String>,

    /// Credit Manager contract address
    pub credit_manager: String,
}

#[cw_serde]
pub enum ExtensionExecuteMsg {
    /// Bind Credit Manager account id to the vault
    BindCreditManagerAccount {
        account_id: String,
    },
}

#[cw_serde]
pub enum ExtensionQueryMsg {
    VaultInfo,
}

#[cw_serde]
pub struct VaultInfoResponseExt {
    /// The token that is accepted for deposits, withdrawals and used for
    /// accounting in the vault. The denom is a native token
    pub base_token: String,
    /// Vault token denom
    pub vault_token: String,

    /// Optional metadata
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub description: Option<String>,

    /// Credit Manager contract address
    pub credit_manager: String,

    /// Vault account id
    pub vault_account_id: Option<String>,
}
