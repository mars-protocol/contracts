use cosmwasm_std::{to_json_binary, Binary, ContractResult, QuerierResult};
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin,
    osmosis::cosmwasmpool::v1beta1::{CalcOutAmtGivenInRequest, CalcOutAmtGivenInResponse},
};

#[derive(Default)]
pub struct CosmWasmPoolQuerier {}

impl CosmWasmPoolQuerier {
    pub fn handle_query(&self, query: CalcOutAmtGivenInRequest) -> QuerierResult {
        let res: ContractResult<Binary> = {
            let token_in = query.calc_out_amt_given_in.clone().unwrap().token_in.unwrap();
            let denom_out = query.calc_out_amt_given_in.unwrap().token_out_denom;

            to_json_binary(&CalcOutAmtGivenInResponse {
                token_out: Some(Coin {
                    denom: denom_out,
                    amount: token_in.amount,
                }),
            })
            .into()
        };
        Ok(res).into()
    }
}
