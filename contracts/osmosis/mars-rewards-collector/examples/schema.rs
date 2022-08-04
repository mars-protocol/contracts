use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use mars_outpost::rewards_collector::{InstantiateMsg, QueryMsg};
use mars_rewards_collector_osmosis::msg::{ExecuteMsg, RouteResponse, RoutesResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(RouteResponse), &out_dir);
    export_schema(&schema_for!(RoutesResponse), &out_dir);
}
