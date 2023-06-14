use injective_std::types::injective::oracle::v1beta1::{
    MsgRelayPriceFeedPrice, MsgRelayPriceFeedPriceResponse, MsgRelayPythPrices,
    MsgRelayPythPricesResponse, QueryModuleStateRequest, QueryModuleStateResponse,
    QueryOraclePriceRequest, QueryOraclePriceResponse, QueryPythPriceRequest,
    QueryPythPriceResponse,
};
use test_tube_inj::module::Module;
use test_tube_inj::runner::Runner;
use test_tube_inj::{fn_execute, fn_query};

pub struct Oracle<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Oracle<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Oracle<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub relay_price_feed: MsgRelayPriceFeedPrice => MsgRelayPriceFeedPriceResponse
    }

    fn_execute! {
        pub relay_pyth_prices: MsgRelayPythPrices => MsgRelayPythPricesResponse
    }

    fn_query! {
        pub query_module_state ["/injective.oracle.v1beta1.Query/OracleModuleState"]: QueryModuleStateRequest => QueryModuleStateResponse
    }

    fn_query! {
        pub query_oracle_price ["/injective.oracle.v1beta1.Query/OraclePrice"]: QueryOraclePriceRequest => QueryOraclePriceResponse
    }

    fn_query! {
        pub query_pyth_price ["/injective.oracle.v1beta1.Query/PythPrice"]: QueryPythPriceRequest => QueryPythPriceResponse
    }
}

#[cfg(test)]
mod tests {
    // use cosmrs::proto::cosmos::params;
    // use cosmrs::proto::cosmos::params::v1beta1::ParamChange;
    use cosmwasm_std::Coin;
    // use injective_std::types::cosmos::gov;
    // use injective_std::types::injective::oracle::v1beta1::{
    //     MsgRelayPythPrices, PriceAttestation, PythPriceState, QueryPythPriceStatesResponse,
    // };
    use injective_std::{
        shim::Any,
        types::{
            cosmos::{
                bank::v1beta1::MsgSend,
                base::v1beta1::Coin as TubeCoin,
                gov::v1beta1::{MsgSubmitProposal, MsgVote},
            },
            injective::oracle,
            injective::oracle::v1beta1::{
                GrantPriceFeederPrivilegeProposal, MsgRelayPriceFeedPrice,
            },
        },
    };
    use prost::Message;
    use std::str::FromStr;
    // use std::time;
    // use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{Account, Bank, Gov, InjectiveTestApp, Module, Oracle};

    #[test]
    fn price_feed_oracle_integration() {
        let app = InjectiveTestApp::new();

        let gov = Gov::new(&app);
        let bank = Bank::new(&app);
        let oracle = Oracle::new(&app);

        let signer = app
            .init_account(&[
                Coin::new(100_000_000_000_000_000_000_000u128, "inj"),
                Coin::new(100_000_000_000_000_000_000u128, "usdt"),
            ])
            .unwrap();

        let mut buf = vec![];
        oracle::v1beta1::GrantPriceFeederPrivilegeProposal::encode(
            &GrantPriceFeederPrivilegeProposal {
                title: "test-proposal".to_string(),
                description: "test-proposal".to_string(),
                base: "inj".to_string(),
                quote: "usdt".to_string(),
                relayers: vec![signer.address()],
            },
            &mut buf,
        )
        .unwrap();

        let validator = app
            .get_first_validator_signing_account("inj".to_string(), 1.2f64)
            .unwrap();

        // fund the validator account
        bank.send(
            MsgSend {
                from_address: signer.address(),
                to_address: validator.address(),
                amount: vec![TubeCoin {
                    amount: "1000000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }],
            },
            &signer,
        )
        .unwrap();

        let res = gov
            .submit_proposal(
                MsgSubmitProposal {
                    content: Some(Any {
                        type_url: "/injective.oracle.v1beta1.GrantPriceFeederPrivilegeProposal"
                            .to_string(),
                        value: buf,
                    }),
                    initial_deposit: vec![TubeCoin {
                        amount: "100000000000000000000".to_string(),
                        denom: "inj".to_string(),
                    }],
                    proposer: validator.address(),
                },
                &validator,
            )
            .unwrap();

        let proposal_id = res
            .events
            .iter()
            .find(|e| e.ty == "submit_proposal")
            .unwrap()
            .attributes[0]
            .value
            .clone();

        gov.vote(
            MsgVote {
                proposal_id: u64::from_str(&proposal_id).unwrap(),
                voter: validator.address(),
                option: 1i32,
            },
            &validator,
        )
        .unwrap();

        // NOTE: increase the block time in order to move past the voting period
        app.increase_time(10u64);

        let expected_price = "12000".to_string();

        oracle
            .relay_price_feed(
                MsgRelayPriceFeedPrice {
                    sender: signer.address(),
                    base: vec!["inj".to_string()],
                    quote: vec!["usdt".to_string()],
                    price: vec![expected_price.clone()],
                },
                &signer,
            )
            .unwrap();

        let price = oracle
            .query_oracle_price(&oracle::v1beta1::QueryOraclePriceRequest {
                oracle_type: 2i32,
                base: "inj".to_string(),
                quote: "usdt".to_string(),
            })
            .unwrap()
            .price_pair_state
            .unwrap()
            .pair_price;

        assert_eq!(price, expected_price, "price should be equal");
    }

    // #[test]
    // fn pyth_oracle_integration() {
    //     let app = InjectiveTestApp::new();
    //     let gov = Gov::new(&app);
    //     let bank = Bank::new(&app);
    //     let oracle = Oracle::new(&app);

    //     let signer = app
    //         .init_account(&[
    //             Coin::new(100_000_000_000_000_000_000_000u128, "inj"),
    //             Coin::new(100_000_000_000_000_000_000u128, "usdt"),
    //         ])
    //         .unwrap();

    //     let validator = app
    //         .get_first_validator_signing_account("inj".to_string(), 1.2f64)
    //         .unwrap();

    //     // fund the validator account
    //     bank.send(
    //         MsgSend {
    //             from_address: signer.address().to_string(),
    //             to_address: validator.address().to_string(),
    //             amount: vec![TubeCoin {
    //                 amount: "1000000000000000000000".to_string(),
    //                 denom: "inj".to_string(),
    //             }],
    //         },
    //         &signer,
    //     )
    //         .unwrap();

    //     let pyth_contract = app
    //         .init_account(&[Coin::new(100_000_000_000_000_000_000_000u128, "inj")])
    //         .unwrap();

    //     let mut buf = vec![];
    //     params::v1beta1::ParameterChangeProposal::encode(
    //         &params::v1beta1::ParameterChangeProposal {
    //             title: "set pyth contract address".to_string(),
    //             description: "set pyth contract address".to_string(),
    //             changes: vec![ParamChange {
    //                 subspace: "oracle".to_string(),
    //                 key: "PythContract".to_string(),
    //                 value: format!("\"{}\"", pyth_contract.address()).to_string(),
    //             }],
    //         },
    //         &mut buf,
    //     )
    //         .unwrap();

    //     let res = gov
    //         .submit_proposal(
    //             MsgSubmitProposal {
    //                 content: Some(Any {
    //                     type_url: "/cosmos.params.v1beta1.ParameterChangeProposal".to_string(),
    //                     value: buf,
    //                 }),
    //                 initial_deposit: vec![TubeCoin {
    //                     amount: "100000000000000000000".to_string(),
    //                     denom: "inj".to_string(),
    //                 }],
    //                 proposer: validator.address().to_string(),
    //             },
    //             &validator,
    //         )
    //         .unwrap();

    //     let proposal_id = res
    //         .events
    //         .iter()
    //         .find(|e| e.ty == "submit_proposal")
    //         .unwrap()
    //         .attributes[0]
    //         .value
    //         .clone();

    //     gov.vote(
    //         MsgVote {
    //             proposal_id: u64::from_str(&proposal_id).unwrap(),
    //             voter: validator.address().to_string(),
    //             option: 1i32,
    //         },
    //         &validator,
    //     )
    //         .unwrap();

    //     // NOTE: increase the block time in order to move past the voting period
    //     app.increase_time(11u64);

    //     let proposal_result = gov.query_proposal(
    //         &gov::v1beta1::QueryProposalRequest {
    //             proposal_id: u64::from_str(&proposal_id).unwrap(),
    //         }
    //     ).unwrap();
    //     println!("{:?}", proposal_result);

    //     let result = oracle.query_module_state(
    //         &oracle::v1beta1::QueryModuleStateRequest {}
    //     ).unwrap();
    //     println!("{:?}", result);

    //     let inj_price_id = "0x7a5bc1d2b56ad029048cd63964b3ad2776eadf812edc1a43a31406cb54bff592";
    //     let usdt_price_id = "0x1fc18861232290221461220bd4e2acd1dcdfbc89c84092c93c18bdc7756c1588";
    //     let now = SystemTime::now();
    //     let unix = now.duration_since(UNIX_EPOCH).unwrap();

    //     let inj_price_attestation = PriceAttestation {
    //         price_id: "inj_price_id".to_string(),
    //         price: 145600,
    //         conf: 500,
    //         expo: -5,
    //         ema_price: 167251,
    //         ema_conf: 2000,
    //         ema_expo: -5,
    //         publish_time: unix.as_millis() as i64,
    //     };

    //     let usdt_price_attestation = PriceAttestation {
    //         price_id: "usdt_price_id".to_string(),
    //         price: 1000,
    //         conf: 500,
    //         expo: -3,
    //         ema_price: 1021,
    //         ema_conf: 2000,
    //         ema_expo: -3,
    //         publish_time: unix.as_millis() as i64,
    //     };

    //     oracle
    //         .relay_pyth_prices(
    //             MsgRelayPythPrices {
    //                 sender: pyth_contract.address().to_string(),
    //                 price_attestations: vec![inj_price_attestation.clone(), usdt_price_attestation.clone()],
    //             },
    //             &pyth_contract,
    //         )
    //         .unwrap();

    //     let inj_price_state = oracle
    //         .query_pyth_price(&oracle::v1beta1::QueryPythPriceRequest {
    //             price_id: inj_price_id.to_string(),
    //         })
    //         .unwrap().price_state.unwrap();

    //     assert!(
    //         inj_price_state.price_state.is_some(),
    //         "inj price state should be some"
    //     );

    //     let inj_price = inj_price_state.price_state.unwrap();
    //     assert_eq!(
    //         inj_price.price,
    //         inj_price_attestation.price.to_string(),
    //         "inj price should be equal to the price attestation"
    //     );
    //     let inj_cumulative_price: i64 = inj_price.cumulative_price.parse().unwrap();
    //     assert!(
    //         inj_cumulative_price > 0,
    //         "inj cumulative price should be greater than 0"
    //     );
    //     assert_eq!(
    //         inj_price_state.conf, inj_price_attestation.conf.to_string(),
    //         "inj conf should be equal to the price attestation"
    //     );
    //     assert_eq!(
    //         inj_price_state.ema_price, inj_price_attestation.ema_price.to_string(),
    //         "inj ema_price should be equal to the price attestation"
    //     );
    //     assert_eq!(
    //         inj_price_state.ema_conf, inj_price_attestation.ema_conf.to_string(),
    //         "inj ema_conf should be equal to the price attestation"
    //     );
    //     assert_eq!(
    //         inj_price_state.publish_time, inj_price_attestation.publish_time as u64,
    //         "inj publish_time should be equal to the price attestation"
    //     );

    //     let usdt_price_state = oracle
    //         .query_pyth_price(&oracle::v1beta1::QueryPythPriceRequest {
    //             price_id: usdt_price_id.to_string(),
    //         })
    //         .unwrap().price_state.unwrap();

    //     assert!(
    //         usdt_price_state.price_state.is_some(),
    //         "usdt price state should be some"
    //     );

    //     let usdt_price = usdt_price_state.price_state.unwrap();
    //     assert_eq!(
    //         usdt_price.price,
    //         usdt_price_attestation.price.to_string(),
    //         "usdt price should be equal to the price attestation"
    //     );
    //     let usdt_cumulative_price: i64 = usdt_price.cumulative_price.parse().unwrap();
    //     assert!(
    //         usdt_cumulative_price > 0,
    //         "usdt cumulative price should be greater than 0"
    //     );
    //     assert_eq!(
    //         usdt_price_state.conf, usdt_price_attestation.conf.to_string(),
    //         "usdt conf should be equal to the price attestation"
    //     );
    //     assert_eq!(
    //         usdt_price_state.ema_price, usdt_price_attestation.ema_price.to_string(),
    //         "usdt ema_price should be equal to the price attestation"
    //     );
    //     assert_eq!(
    //         usdt_price_state.ema_conf, usdt_price_attestation.ema_conf.to_string(),
    //         "usdt ema_conf should be equal to the price attestation"
    //     );
    //     assert_eq!(
    //         usdt_price_state.publish_time, usdt_price_attestation.publish_time as u64,
    //         "usdt publish_time should be equal to the price attestation"
    //     );
    // }
}
