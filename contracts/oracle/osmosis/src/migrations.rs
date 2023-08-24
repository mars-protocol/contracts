/// Migration logic for Oracle contract with version: 1.0.1
pub mod v1_0_1 {
    use cosmwasm_std::{DepsMut, Response};
    use mars_oracle_base::ContractResult;
    use mars_owner::{Owner, OwnerInit};

    use crate::contract::{CONTRACT_NAME, CONTRACT_VERSION};

    const FROM_VERSION: &str = "1.0.1";

    pub fn migrate(deps: DepsMut) -> ContractResult<Response> {
        // make sure we're migrating the correct contract and from the correct version
        cw2::assert_contract_version(
            deps.as_ref().storage,
            &format!("crates.io:{CONTRACT_NAME}"),
            FROM_VERSION,
        )?;

        // map old owner struct to new one
        let old_owner = old_state::OWNER.load(deps.storage)?;
        let owner = match old_owner {
            old_state::OwnerState::B(state) => state.owner.to_string(),
            old_state::OwnerState::C(state) => state.owner.to_string(),
        };

        // clear old owner state
        old_state::OWNER.remove(deps.storage);

        // initalize owner with new struct
        Owner::new("owner").initialize(
            deps.storage,
            deps.api,
            OwnerInit::SetInitialOwner {
                owner,
            },
        )?;

        // update contract version
        cw2::set_contract_version(
            deps.storage,
            format!("crates.io:{CONTRACT_NAME}"),
            CONTRACT_VERSION,
        )?;

        Ok(Response::new()
            .add_attribute("action", "migrate")
            .add_attribute("from_version", FROM_VERSION)
            .add_attribute("to_version", CONTRACT_VERSION))
    }

    pub mod old_state {
        use cosmwasm_schema::cw_serde;
        use cosmwasm_std::Addr;
        use cw_storage_plus::Item;

        pub const OWNER: Item<OwnerState> = Item::new("owner");

        /// Old OwnerState variants:
        /// A(OwnerUninitialized)
        /// B(OwnerSetNoneProposed)
        /// C(OwnerSetWithProposed)
        /// D(OwnerRoleAbolished)
        ///
        /// Oracle contract can be in B or C state. Emergency owner is not supported for this contract.
        /// We can only read `owner` value and omit `proposed` if exist.
        #[cw_serde]
        pub enum OwnerState {
            B(OwnerSetNoneProposed),
            C(OwnerSetWithProposed),
        }

        #[cw_serde]
        pub struct OwnerSetNoneProposed {
            pub owner: Addr,
        }

        #[cw_serde]
        pub struct OwnerSetWithProposed {
            pub owner: Addr,
        }
    }

    #[cfg(test)]
    mod tests {
        use cosmwasm_std::{attr, testing::mock_dependencies, Addr};

        use super::*;
        use crate::migrations::v1_0_1::old_state::{OwnerSetNoneProposed, OwnerSetWithProposed};

        #[test]
        fn migration_owner_from_state_b() {
            let mut deps = mock_dependencies();

            cw2::set_contract_version(
                deps.as_mut().storage,
                format!("crates.io:{CONTRACT_NAME}"),
                FROM_VERSION,
            )
            .unwrap();

            old_state::OWNER
                .save(
                    deps.as_mut().storage,
                    &old_state::OwnerState::B(OwnerSetNoneProposed {
                        owner: Addr::unchecked("xyz"),
                    }),
                )
                .unwrap();

            let res = migrate(deps.as_mut()).unwrap();
            assert_eq!(res.messages, vec![]);
            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "migrate"),
                    attr("from_version", "1.0.1"),
                    attr("to_version", "1.2.0")
                ]
            );

            let new_owner = Owner::new("owner").query(&deps.storage).unwrap();
            assert_eq!(new_owner.owner.unwrap(), "xyz".to_string());
        }

        #[test]
        fn migration_owner_from_state_c() {
            let mut deps = mock_dependencies();

            cw2::set_contract_version(
                deps.as_mut().storage,
                format!("crates.io:{CONTRACT_NAME}"),
                FROM_VERSION,
            )
            .unwrap();

            old_state::OWNER
                .save(
                    deps.as_mut().storage,
                    &old_state::OwnerState::C(OwnerSetWithProposed {
                        owner: Addr::unchecked("xyz"),
                    }),
                )
                .unwrap();

            let res = migrate(deps.as_mut()).unwrap();
            assert_eq!(res.messages, vec![]);
            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "migrate"),
                    attr("from_version", "1.0.1"),
                    attr("to_version", "1.2.0")
                ]
            );

            let new_owner = Owner::new("owner").query(&deps.storage).unwrap();
            assert_eq!(new_owner.owner.unwrap(), "xyz".to_string());
        }
    }
}
