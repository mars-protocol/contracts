use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use mars_council::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use mars_council::{Config, Proposal, ProposalVotesResponse, ProposalsListResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(ReceiveMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(Proposal), &out_dir);
    export_schema(&schema_for!(ProposalsListResponse), &out_dir);
    export_schema(&schema_for!(ProposalVotesResponse), &out_dir);
}
