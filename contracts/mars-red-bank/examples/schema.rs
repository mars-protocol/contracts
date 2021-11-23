use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use mars_red_bank::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use mars_red_bank::{
    ConfigResponse, Market, MarketsListResponse, UserAssetDebtResponse, UserCollateralResponse,
    UserDebtResponse, UserPositionResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ReceiveMsg), &out_dir);

    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(Market), &out_dir);
    export_schema(&schema_for!(MarketsListResponse), &out_dir);
    export_schema(&schema_for!(UserDebtResponse), &out_dir);
    export_schema(&schema_for!(UserAssetDebtResponse), &out_dir);
    export_schema(&schema_for!(UserCollateralResponse), &out_dir);
    export_schema(&schema_for!(UserPositionResponse), &out_dir);
}
