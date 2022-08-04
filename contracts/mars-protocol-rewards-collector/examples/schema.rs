use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use mars_outpost::protocol_rewards_collector::{
    InstantiateMsg, QueryMsg, RouteResponse, RoutesResponse,
};
use mars_protocol_rewards_collector::{msg::ExecuteMsg, Route};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(RouteResponse<Route>), &out_dir);
    export_schema(&schema_for!(RoutesResponse<Route>), &out_dir);
}
