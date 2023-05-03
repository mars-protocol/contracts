use cosmwasm_std::coin;
use mars_v3_zapper_base::msg::NewPositionRequest;

pub fn default_new_position_req() -> NewPositionRequest {
    NewPositionRequest {
        pool_id: 1,
        lower_tick: -1,
        upper_tick: 100,
        token_desired0: Some(coin(100_000_000, "ujuno")),
        token_desired1: Some(coin(100_000_000, "umars")),
        token_min_amount0: "10000".to_string(),
        token_min_amount1: "10000".to_string(),
    }
}
