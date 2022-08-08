use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

fn main() {
    address_provider();
    incentives();
    red_bank();
    oracle_osmosis();
    rewards_collector_osmosis();
}

fn address_provider() {
    use mars_outpost::address_provider::{
        AddressResponseItem, ExecuteMsg, InstantiateMsg, QueryMsg,
    };

    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema/schema/mars_address_provider");

    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(AddressResponseItem), &out_dir);
}

fn incentives() {
    use mars_outpost::incentives::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use mars_outpost::incentives::{AssetIncentiveResponse, Config};

    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema/schema/mars_incentives");

    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(AssetIncentiveResponse), &out_dir);
}

fn oracle_osmosis() {
    use mars_oracle_osmosis::msg::{ExecuteMsg, PriceSourceResponse};
    use mars_outpost::oracle::{Config, InstantiateMsg, PriceResponse, QueryMsg};

    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema/schema/mars_oracle_osmosis");

    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    export_schema(&schema_for!(Config<String>), &out_dir);
    export_schema(&schema_for!(PriceSourceResponse), &out_dir);
    export_schema(&schema_for!(PriceResponse), &out_dir);
}

fn red_bank() {
    use mars_outpost::red_bank::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
    use mars_outpost::red_bank::{
        ConfigResponse, Market, MarketsListResponse, UserAssetDebtResponse, UserCollateralResponse,
        UserDebtResponse, UserPositionResponse,
    };

    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema/schema/mars_red_bank");

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

fn rewards_collector_osmosis() {
    use mars_outpost::rewards_collector::{InstantiateMsg, QueryMsg};
    use mars_rewards_collector_osmosis::msg::{ExecuteMsg, RouteResponse, RoutesResponse};

    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema/schema/mars_rewards_collector_osmosis");

    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(RouteResponse), &out_dir);
    export_schema(&schema_for!(RoutesResponse), &out_dir);
}
