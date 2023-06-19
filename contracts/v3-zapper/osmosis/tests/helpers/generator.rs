use cosmwasm_std::coin;
use mars_v3_zapper_base::msg::NewPositionRequest;

pub fn default_new_position_req() -> NewPositionRequest {
    NewPositionRequest {
        pool_id: 1,
        lower_tick: -1,
        upper_tick: 100,
        token_min_amount0: "10000".to_string(),
        token_min_amount1: "10000".to_string(),
        tokens_provided: vec![coin(100_000_000, "ujuno"), coin(100_000_000, "umars")],
    }
}
