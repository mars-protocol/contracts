/// Migration logic for Oracle contract with version: 1.0.0
pub mod v1_0_0 {
    use cosmwasm_std::{DepsMut, Response};
    use mars_oracle_base::ContractResult;

    use crate::contract::{CONTRACT_NAME, CONTRACT_VERSION};

    const FROM_VERSION: &str = "1.0.0";

    pub fn migrate(deps: DepsMut) -> ContractResult<Response> {
        // make sure we're migrating the correct contract and from the correct version
        cw2::assert_contract_version(deps.as_ref().storage, CONTRACT_NAME, FROM_VERSION)?;

        // update contract version
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        Ok(Response::new()
            .add_attribute("action", "migrate")
            .add_attribute("from_version", FROM_VERSION)
            .add_attribute("to_version", CONTRACT_VERSION))
    }

    #[cfg(test)]
    mod tests {
        use cosmwasm_std::{attr, testing::mock_dependencies};

        use super::*;

        #[test]
        fn proper_migration() {
            let mut deps = mock_dependencies();

            cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, FROM_VERSION).unwrap();

            let res = migrate(deps.as_mut()).unwrap();
            assert_eq!(res.messages, vec![]);
            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "migrate"),
                    attr("from_version", "1.0.0"),
                    attr("to_version", "1.0.1")
                ]
            );
        }
    }
}
