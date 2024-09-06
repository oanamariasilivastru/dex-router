#![allow(non_snake_case)]

mod proxy;

use err_msg::FORMATTER_ENCODE_ERROR;
use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::sdk;
use multiversx_sc_snippets::sdk::data::address;
use serde::{Deserialize, Serialize};
use std::f32::INFINITY;
use std::future::IntoFuture;
use std::str::FromStr;
use std::{
    io::{Read, Write},
    path::Path,
};


const GATEWAY: &str = sdk::blockchain::DEVNET_GATEWAY;
const STATE_FILE: &str = "state.toml";
const BOB_ADDRESS: &str = "erd1spyavw0956vq68xj8y4tenjpq2wd5a9p2c6j8gsz7ztyrnpxrruqzu66jx";
const ALICE_ADDRESS: &str = "erd1qyu5wthldzr8wx5c9ucg8kjagg0jfs53s8nr3zpz3hypefsdd8ssycr6th";
const FIRST_TOKEN_ID: &str = "TSTT-d96162";
const SECOND_TOKEN_ID: &str = "TST-af9b21";
const THIRD_TOKEN_ID: &str = "TEST-248836";
const FOURTH_TOKEN_ID: &str = "TEST-49b549";
const LP_TOKEN_NAME: &str = "LPTOKEN";
const LP_TOKEN_TICKER: &str = "LPTT";
const LP_TOKEN_INITIAL_SUPPLY: u128 = 1000;
const LP_TOKEN_INIT_DECIMALS: u128 = 1000000000000000000000;
const SWAP_TOKENS_FIXED_INPUT_FUNC_NAME: &[u8] = b"swapTokensFixedInput";
const SWAP_TOKENS_FIXED_OUTPUT_FUNC_NAME: &[u8] = b"swapTokensFixedOutput";


#[tokio::main]
async fn main() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let mut interact = ContractInteract::new().await;
    match cmd.as_str() {
        "deploy" => interact.deploy().await,
        "pause" => interact.pause().await,
        "resume" => interact.resume().await,
        "createPair" => interact.create_pair_endpoint().await,
        "upgradePair" => interact.upgrade_pair_endpoint().await,
        "issueLpToken" => interact.issue_lp_token().await,
        "setLocalRoles" => interact.set_local_roles().await,
        "removePair" => interact.remove_pair().await,
        "setFeeOn" => interact.set_fee_on().await,
        "setFeeOff" => interact.set_fee_off().await,
        "setPairCreationEnabled" => interact.set_pair_creation_enabled().await,
        "getPairCreationEnabled" => interact.pair_creation_enabled().await,
        "getState" => interact.state().await,
        "getOwner" => interact.owner().await,
        "setTemporaryOwnerPeriod" => interact.set_temporary_owner_period().await,
        "setPairTemplateAddress" => interact.set_pair_template_address().await,
        "getPairTemplateAddress" => interact.pair_template_address().await,
        "getTemporaryOwnerPeriod" => interact.temporary_owner_period().await,
        "getCommonTokensForUserPairs" => interact.common_tokens_for_user_pairs().await,
        "getAllPairsManagedAddresses" => interact.get_all_pairs_addresses().await,
        "getAllPairTokens" => interact.get_all_token_pairs().await,
        "getAllPairContractMetadata" => interact.get_all_pair_contract_metadata().await,
        "getPair" => interact.get_pair().await,
        "clearPairTemporaryOwnerStorage" => interact.clear_pair_temporary_owner_storage().await,
        "multiPairSwap" => interact.multi_pair_swap().await,
        "configEnableByUserParameters" => interact.config_enable_by_user_parameters().await,
        "addCommonTokensForUserPairs" => interact.add_common_tokens_for_user_pairs().await,
        "removeCommonTokensForUserPairs" => interact.remove_common_tokens_for_user_pairs().await,
        "setSwapEnabledByUser" => interact.set_swap_enabled_by_user().await,
        "getEnableSwapByUserConfig" => interact.try_get_config().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}


#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    contract_address: Option<Bech32Address>
}

impl State {
        // Deserializes state from file
        pub fn load_state() -> Self {
            if Path::new(STATE_FILE).exists() {
                let mut file = std::fs::File::open(STATE_FILE).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content).unwrap();
                toml::from_str(&content).unwrap()
            } else {
                Self::default()
            }
        }
    
        /// Sets the contract address
        pub fn set_address(&mut self, address: Bech32Address) {
            self.contract_address = Some(address);
        }
    
        /// Returns the contract address
        pub fn current_address(&self) -> &Bech32Address {
            self.contract_address
                .as_ref()
                .expect("no known contract, deploy first")
        }
    }
    
    impl Drop for State {
        // Serializes state to file
        fn drop(&mut self) {
            let mut file = std::fs::File::create(STATE_FILE).unwrap();
            file.write_all(toml::to_string(self).unwrap().as_bytes())
                .unwrap();
        }
    }

struct ContractInteract {
    interactor: Interactor,
    wallet_address: Address,
    contract_code: BytesValue,
    state: State
}

impl ContractInteract {
    async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let wallet_address = interactor.register_wallet(test_wallets::alice());
        
        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/router.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            wallet_address,
            contract_code,
            state: State::load_state()
        }
    }

    async fn deploy(&mut self) {
        let pair_template_address_opt = OptionalValue::Some(bech32::decode("erd1qqqqqqqqqqqqqpgq5ahfdjfs84fxl7wtvke8us6m5l7lejl7d8sscflcgm"));

        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .init(pair_template_address_opt)
            .code(&self.contract_code)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state
            .set_address(Bech32Address::from_bech32_string(new_address_bech32.clone()));

        println!("new address: {new_address_bech32}");
    }

    async fn pause(&mut self) {
        let address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .pause(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn pause_with_params(&mut self, address: Address) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .pause(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
    async fn pause_with_params_fail(&mut self, address: Address, expected_result: ExpectError<'_>) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .pause(address)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn resume(&mut self) {
        let address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .resume(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn resume_with_params(&mut self, address: Address) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .resume(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
    async fn create_pair_endpoint(&mut self) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(FIRST_TOKEN_ID);
        let second_token_id = TokenIdentifier::from_esdt_bytes(SECOND_TOKEN_ID);
        // let initial_liquidity_adder = bech32::decode("erd1qqqqqqqqqqqqqpgq5ahfdjfs84fxl7wtvke8us6m5l7lejl7d8sscflcgm");
        let initial_liquidity_adder = Address::zero();
        // let initial_liquidity_adder = bech32::decode("erd1qqqqqqqqqqqqqpgqnh9zxrwuesevwcvvdqx6fshfc25aus5qd8sswq4d0g");
        let opt_fee_percents = OptionalValue::Some(MultiValue2::from((100u64, 3u64)));
        let admins = MultiValueVec::from(vec![bech32::decode(BOB_ADDRESS), bech32::decode(ALICE_ADDRESS)]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(99_000_000)
            .typed(proxy::RouterProxy)
            .create_pair_endpoint(first_token_id, second_token_id, initial_liquidity_adder, opt_fee_percents, admins)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn create_pair_endpoint_with_params(&mut self, first_token: &str, second_token: &str, fee_percents: u64, admins: MultiValueVec<Address>) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(first_token);
        let second_token_id = TokenIdentifier::from_esdt_bytes(second_token);
        // let initial_liquidity_adder = bech32::decode("erd1qqqqqqqqqqqqqpgq5ahfdjfs84fxl7wtvke8us6m5l7lejl7d8sscflcgm");
        let initial_liquidity_adder = Address::zero();
        // let initial_liquidity_adder = bech32::decode("erd1qqqqqqqqqqqqqpgqnh9zxrwuesevwcvvdqx6fshfc25aus5qd8sswq4d0g");
        let opt_fee_percents = OptionalValue::Some(MultiValue2::from((fee_percents, 3u64)));
        // let admins = MultiValueVec::from(vec![bech32::decode(BOB_ADDRESS), bech32::decode(ALICE_ADDRESS)]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(99_000_000)
            .typed(proxy::RouterProxy)
            .create_pair_endpoint(first_token_id, second_token_id, initial_liquidity_adder, opt_fee_percents, admins)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
    async fn create_pair_endpoint_fail(&mut self, expected_result: ExpectError<'_>) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(FIRST_TOKEN_ID);
        let second_token_id = TokenIdentifier::from_esdt_bytes(SECOND_TOKEN_ID);
        // let initial_liquidity_adder = bech32::decode("erd1qqqqqqqqqqqqqpgq5ahfdjfs84fxl7wtvke8us6m5l7lejl7d8sscflcgm");
        let initial_liquidity_adder = Address::zero();
        // let initial_liquidity_adder = bech32::decode("erd1qqqqqqqqqqqqqpgqnh9zxrwuesevwcvvdqx6fshfc25aus5qd8sswq4d0g");
        let opt_fee_percents = OptionalValue::Some(MultiValue2::from((100u64, 3u64)));
        let admins = MultiValueVec::from(vec![bech32::decode(BOB_ADDRESS), bech32::decode(ALICE_ADDRESS)]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(99_000_000)
            .typed(proxy::RouterProxy)
            .create_pair_endpoint(first_token_id, second_token_id, initial_liquidity_adder, opt_fee_percents, admins)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn upgrade_pair_endpoint(&mut self) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(&b""[..]);
        let second_token_id = TokenIdentifier::from_esdt_bytes(&b""[..]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .upgrade_pair_endpoint(first_token_id, second_token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn upgrade_pair_endpoint_with_params(&mut self, first_token: &str, second_token: &str) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(first_token);
        let second_token_id = TokenIdentifier::from_esdt_bytes(second_token);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .upgrade_pair_endpoint(first_token_id, second_token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    
    async fn upgrade_pair_endpoint_with_params_fail(&mut self, first_token: &str, second_token: &str, expected_result: ExpectError<'_>) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(first_token);
        let second_token_id = TokenIdentifier::from_esdt_bytes(second_token);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .upgrade_pair_endpoint(first_token_id, second_token_id)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
    
    async fn issue_lp_token(&mut self) {
        let egld_amount = BigUint::<StaticApi>::from(0u128);

        let pair_address = bech32::decode("");
        let lp_token_display_name = ManagedBuffer::new_from_bytes(&b""[..]);
        let lp_token_ticker = ManagedBuffer::new_from_bytes(&b""[..]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .issue_lp_token(pair_address, lp_token_display_name, lp_token_ticker)
            .egld(egld_amount)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn issue_lp_token_with_params(&mut self, amount: u128, address: Address, token_name: &str, token_ticker: &str) {
        let egld_amount = BigUint::<StaticApi>::from(50000000000000000u64);
        // let egld_amount = BigUint::<StaticApi>::from(LP_TOKEN_INITIAL_SUPPLY);

        let pair_address = address;
        let lp_token_display_name = ManagedBuffer::new_from_bytes(token_name.as_bytes());
        let lp_token_ticker = ManagedBuffer::new_from_bytes(token_ticker.as_bytes());

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .issue_lp_token(pair_address, lp_token_display_name, lp_token_ticker)
            .egld(egld_amount)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn issue_lp_token_with_params_fail(&mut self, amount: u128, address: Address, token_name: &str, token_ticker: &str, expected_result: ExpectError<'_>) {
        let egld_amount = BigUint::<StaticApi>::from(50000000000000000u64);
        // let egld_amount = BigUint::<StaticApi>::from(LP_TOKEN_INITIAL_SUPPLY);

        let pair_address = address;
        let lp_token_display_name = ManagedBuffer::new_from_bytes(token_name.as_bytes());
        let lp_token_ticker = ManagedBuffer::new_from_bytes(token_ticker.as_bytes());

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .issue_lp_token(pair_address, lp_token_display_name, lp_token_ticker)
            .egld(egld_amount)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_local_roles(&mut self) {
        let pair_address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .set_local_roles(pair_address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_local_roles_with_params(&mut self, address: Address) {
        let pair_address = address;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .set_local_roles(pair_address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_local_roles_with_params_fail(&mut self, address: Address, expected_result: ExpectError<'_>) {
        let pair_address = address;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .set_local_roles(pair_address)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }


    async fn remove_pair(&mut self) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(&b""[..]);
        let second_token_id = TokenIdentifier::from_esdt_bytes(&b""[..]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .remove_pair(first_token_id, second_token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_pair_with_params(&mut self, first_token: &str, second_token: &str) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(first_token);
        let second_token_id = TokenIdentifier::from_esdt_bytes(second_token);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .remove_pair(first_token_id, second_token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_pair_with_params_fail(&mut self, first_token: &str, second_token: &str, expected_result: ExpectError<'_>) {
        let first_token_id = TokenIdentifier::from_esdt_bytes(first_token);
        let second_token_id = TokenIdentifier::from_esdt_bytes(second_token);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .remove_pair(first_token_id, second_token_id)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_fee_on(&mut self) {
        let pair_address = bech32::decode("");
        let fee_to_address = bech32::decode("");
        let fee_token = TokenIdentifier::from_esdt_bytes(&b""[..]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .set_fee_on(pair_address, fee_to_address, fee_token)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_fee_on_with_params(&mut self, address: Address, fee: &str, token: &str) {
        let pair_address = address;
        let fee_to_address = bech32::decode(fee);
        let fee_token = TokenIdentifier::from_esdt_bytes(token);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .set_fee_on(pair_address, fee_to_address, fee_token)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_fee_off(&mut self) {
        let pair_address = bech32::decode("");
        let fee_to_address = bech32::decode("");
        let fee_token = TokenIdentifier::from_esdt_bytes(&b""[..]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .set_fee_off(pair_address, fee_to_address, fee_token)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    
    async fn set_fee_off_with_params(&mut self, address: Address, fee: &str, token: &str) {
        let pair_address= address;
        let fee_to_address = bech32::decode(fee);
        let fee_token = TokenIdentifier::from_esdt_bytes(token);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .set_fee_off(pair_address, fee_to_address, fee_token)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_pair_creation_enabled(&mut self) {
        // let enabled = PlaceholderInput;
        let enabled = true;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .set_pair_creation_enabled(enabled)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn pair_creation_enabled(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .pair_creation_enabled()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn state(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .state()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn owner(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .owner()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn set_temporary_owner_period(&mut self) {
        let period_blocks = 0u64;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .set_temporary_owner_period(period_blocks)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_pair_template_address(&mut self) {
        // let contract_address = self.state.current_address().clone();
        // let contract_address_bech32_str = contract_address.to_bech32_str();
        // let address = bech32::decode(&contract_address_bech32_str);

        let address = bech32::decode("erd1qqqqqqqqqqqqqpgq3n2geaxmeg4yelfglfhu8l6qdtd0pjkyd8ss2gadwa");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .set_pair_template_address(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn pair_template_address(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .pair_template_address()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn temporary_owner_period(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .temporary_owner_period()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn common_tokens_for_user_pairs(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .common_tokens_for_user_pairs()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_all_pairs_addresses(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .get_all_pairs_addresses()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_all_token_pairs(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .get_all_token_pairs()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_all_pair_contract_metadata(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .get_all_pair_contract_metadata()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_pair(&mut self){
        let first_token_id = TokenIdentifier::from_esdt_bytes(FIRST_TOKEN_ID);
        let second_token_id = TokenIdentifier::from_esdt_bytes(SECOND_TOKEN_ID);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .get_pair(first_token_id, second_token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_pair_with_params(&mut self, first_token: &str, second_token: &str) -> Address {
        let first_token_id = TokenIdentifier::from_esdt_bytes(first_token);
        let second_token_id = TokenIdentifier::from_esdt_bytes(second_token);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .get_pair(first_token_id, second_token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
        result_value
    }

    async fn clear_pair_temporary_owner_storage(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .clear_pair_temporary_owner_storage()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn multi_pair_swap(&mut self) {
        let token_id = String::new();
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(0u128);

        let swap_operations = MultiValueVec::from(vec![MultiValue4::from((bech32::decode(""), ManagedBuffer::new_from_bytes(&b""[..]), TokenIdentifier::from_esdt_bytes(&b""[..]), BigUint::<StaticApi>::from(0u128)))]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .multi_pair_swap(swap_operations)
            .payment((TokenIdentifier::from(token_id.as_str()), token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn multi_pair_swap_with_params(
        &mut self, 
        token: &str, 
        nonce: u64, 
        amount: u128, 
        pair_address: Address, 
        function_name: &[u8], 
        wanted_token: &str, 
        wanted_amount: u128
    ) {
        let token_id = token;
        let token_nonce = nonce;
        let token_amount = BigUint::<StaticApi>::from(amount);

        let swap_operations = MultiValueVec::from(vec![MultiValue4::from((pair_address, ManagedBuffer::new_from_bytes(function_name), TokenIdentifier::from_esdt_bytes(wanted_token), BigUint::<StaticApi>::from(wanted_amount)))]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .multi_pair_swap(swap_operations)
            .payment((TokenIdentifier::from(token_id), token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn multi_pair_swap_with_params_fail(
        &mut self, 
        token: &str, 
        nonce: u64, 
        amount: u128, 
        pair_address: Address, 
        function_name: &[u8], 
        wanted_token: &str,
        wanted_amount: u128,
        expected_result: ExpectError<'_>,
    ) {
        let token_id = token;
        let token_nonce = nonce;
        let token_amount = BigUint::<StaticApi>::from(amount);

        let swap_operations = MultiValueVec::from(vec![MultiValue4::from((pair_address, ManagedBuffer::new_from_bytes(function_name), TokenIdentifier::from_esdt_bytes(wanted_token), BigUint::<StaticApi>::from(wanted_amount)))]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(90_000_000)
            .typed(proxy::RouterProxy)
            .multi_pair_swap(swap_operations)
            .payment((TokenIdentifier::from(token_id), token_nonce, token_amount))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn config_enable_by_user_parameters(&mut self) {
        let common_token_id = TokenIdentifier::from_esdt_bytes(&b""[..]);
        let locked_token_id = TokenIdentifier::from_esdt_bytes(&b""[..]);
        let min_locked_token_value = BigUint::<StaticApi>::from(0u128);
        let min_lock_period_epochs = 0u64;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .config_enable_by_user_parameters(common_token_id, locked_token_id, min_locked_token_value, min_lock_period_epochs)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn add_common_tokens_for_user_pairs(&mut self) {
        let tokens = MultiValueVec::from(vec![TokenIdentifier::from_esdt_bytes(&b""[..])]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .add_common_tokens_for_user_pairs(tokens)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_common_tokens_for_user_pairs(&mut self) {
        let tokens = MultiValueVec::from(vec![TokenIdentifier::from_esdt_bytes(&b""[..])]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .remove_common_tokens_for_user_pairs(tokens)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_swap_enabled_by_user(&mut self) {
        let token_id = String::new();
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(0u128);

        let pair_address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .set_swap_enabled_by_user(pair_address)
            .payment((TokenIdentifier::from(token_id.as_str()), token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn try_get_config(&mut self) {
        let token_id = TokenIdentifier::from_esdt_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .try_get_config(token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn try_get_config_with_params(&mut self, token_name: &str) {
        let token_id = TokenIdentifier::from_esdt_bytes(token_name);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::RouterProxy)
            .try_get_config(token_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }
}

#[tokio::test]
async fn test_deploy() {
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
}

#[tokio::test]
async fn test_set_template() {
    let mut interact = ContractInteract::new().await;
    
    interact.set_pair_template_address().await;
    interact.pair_template_address().await;
}

#[tokio::test]
async fn test_pair() {
    let mut interact = ContractInteract::new().await;
    
    interact.create_pair_endpoint().await;
    interact.get_pair().await;
}

#[tokio::test]
async fn test_pair_fail() {
    let mut interact = ContractInteract::new().await;
    
    interact.create_pair_endpoint_fail(ExpectError(4, "Pair already exists")).await;
}

#[tokio::test]
async fn test_get_pair() {
    let mut interact = ContractInteract::new().await;
    
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");
}

#[tokio::test]
async fn test_issue_lp_token() {
    let mut interact = ContractInteract::new().await;
    
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");
    
    interact.issue_lp_token_with_params(
        LP_TOKEN_INITIAL_SUPPLY, 
        result, LP_TOKEN_NAME, 
        LP_TOKEN_TICKER
    ).await;
}

#[tokio::test]
async fn test_issue_lp_token_fail() {
    let mut interact = ContractInteract::new().await;
    
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");
    
    interact.issue_lp_token_with_params_fail(
        LP_TOKEN_INITIAL_SUPPLY, 
        result, 
        LP_TOKEN_NAME, 
        LP_TOKEN_TICKER, 
        ExpectError(4, "LP Token already issued")
    ).await;
}

#[tokio::test]
async fn test_issue_lp_token_fail_disabled() {
    let mut interact = ContractInteract::new().await;
    
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");
    
    interact.pair_creation_enabled().await;
    // interact.issue_lp_token_with_params_fail(LP_TOKEN_INITIAL_SUPPLY, result, LP_TOKEN_NAME, LP_TOKEN_TICKER, ExpectError(4, "LP Token already issued")).await;
}

#[tokio::test]
async fn test_token_fail() {
    let mut interact = ContractInteract::new().await;
    
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");
    
    interact.issue_lp_token_with_params_fail(
        LP_TOKEN_INITIAL_SUPPLY, 
        result, 
        LP_TOKEN_NAME, 
        LP_TOKEN_TICKER, 
        ExpectError(4, "LP Token already issued")
    ).await;
}

#[tokio::test]
async fn test_pause_resume() {
    let mut interact = ContractInteract::new().await;
    
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");
    
    interact.issue_lp_token_with_params_fail(
        LP_TOKEN_INITIAL_SUPPLY, 
        result.clone(), 
        LP_TOKEN_NAME, 
        LP_TOKEN_TICKER, 
        ExpectError(4, "LP Token already issued")
    ).await;
    
    interact.pause_with_params(result.clone()).await;
    
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result after pause: {result:?}");
    
    interact.issue_lp_token_with_params_fail(
        LP_TOKEN_INITIAL_SUPPLY, 
        result.clone(), 
        LP_TOKEN_NAME, 
        LP_TOKEN_TICKER, 
        ExpectError(4, "LP Token already issued")
    ).await;
    
    interact.resume_with_params(result).await;
}

#[tokio::test]
async fn test_pause(){
    let mut interact = ContractInteract::new().await;
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");
    let alice_address = bech32::decode(ALICE_ADDRESS);

    interact.pause_with_params_fail(
        alice_address, 
        ExpectError(4, "Not a pair SC")
    ).await;

}

#[tokio::test]
async fn test_upgrade_pair(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;

    interact.set_pair_template_address().await;
    interact.pair_template_address().await;

    interact.create_pair_endpoint().await;
    interact.get_pair().await;

    let first_token: &str = "TST-c0986b";
    let second_token: &str = "TSTT-d96162";
    let fee_percents = 100u64;
    let admins = MultiValueVec::from(vec![bech32::decode(BOB_ADDRESS), bech32::decode(ALICE_ADDRESS)]);

    interact.create_pair_endpoint_with_params(
        first_token, 
        second_token, 
        fee_percents, 
        admins
    ).await;

    interact.get_all_pair_contract_metadata().await;
    interact.get_all_pairs_addresses().await;
    interact.get_all_token_pairs().await;

    interact.upgrade_pair_endpoint_with_params(
        first_token, 
        second_token
    ).await;

    interact.get_all_pair_contract_metadata().await;
    interact.get_all_pairs_addresses().await;
    interact.get_all_token_pairs().await;
}

#[tokio::test]
async fn upgrade_fail(){
    let mut interact = ContractInteract::new().await;

    let first_token: &str = "ALICE";
    let second_token: &str = "TSTT-d96162";
    let fee_percents = 100u64;
    let admins = MultiValueVec::from(vec![bech32::decode(BOB_ADDRESS), bech32::decode(ALICE_ADDRESS)]);
    
    interact.upgrade_pair_endpoint_with_params_fail(
        first_token, 
        second_token, 
        ExpectError(4, "First Token ID is not a valid esdt token ID")
    ).await;

    let first_token: &str = "TSTT-d96162";
    let second_token: &str = "ALICE";

    interact.upgrade_pair_endpoint_with_params_fail(
        first_token, 
        second_token, 
        ExpectError(4, "Second Token ID is not a valid esdt token ID")
    ).await;
}

#[tokio::test]
async fn upgrade_fail_pair(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;

    let first_token: &str = "TST-c0986b";
    let second_token: &str = "TSTT-d96162";

    interact.upgrade_pair_endpoint_with_params_fail(
        first_token, 
        second_token, 
        ExpectError(4, "Pair does not exists")
    ).await;
}

#[tokio::test]
async fn test_set_roles(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;

    interact.set_pair_template_address().await;
    interact.pair_template_address().await;

    interact.create_pair_endpoint().await;
       
    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");
    
    interact.set_local_roles_with_params_fail(
        result.clone(), 
        ExpectError(4, "LP token not issued")
    ).await;

    interact.issue_lp_token_with_params(
        LP_TOKEN_INITIAL_SUPPLY, 
        result.clone(), 
        LP_TOKEN_NAME, 
        LP_TOKEN_TICKER
    ).await;

    interact.set_local_roles_with_params(result).await;

}

#[tokio::test]
async fn test_issue_lp_token_sample(){
    let mut interact = ContractInteract::new().await;

    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;
    println!("Result: {result:?}");

    interact.issue_lp_token_with_params(
        LP_TOKEN_INITIAL_SUPPLY, 
        result, 
        LP_TOKEN_NAME, 
        LP_TOKEN_TICKER
    ).await;;
}

#[tokio::test]
async fn test_issue_set_local_roles(){
    let mut interact = ContractInteract::new().await;

    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");

    interact.set_local_roles_with_params(result).await;
}
#[tokio::test]
async fn test_remove_pair(){
    let mut interact= ContractInteract::new().await;

    interact.deploy().await;
    interact.set_pair_template_address().await;

    let first_token: &str = "ALICE";
    let second_token: &str = "TSTT-d96162";

    interact.remove_pair_with_params_fail(
        first_token, 
        second_token, 
        ExpectError(4, "First Token ID is not a valid esdt token ID")
    ).await;

    interact.remove_pair_with_params_fail(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID, 
        ExpectError(4, "Pair does not exists")
    ).await;

    interact.create_pair_endpoint().await;

    interact.remove_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    interact.remove_pair_with_params_fail(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID, 
        ExpectError(4, "Pair does not exists")
    ).await;
    
}

#[tokio::test]
async fn test_set_fee(){
    let mut interact= ContractInteract::new().await;

    interact.deploy().await;
    interact.set_pair_template_address().await;

    interact.create_pair_endpoint().await;

    let result = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    println!("Result: {result:?}");

    interact.set_fee_on_with_params(result.clone(), ALICE_ADDRESS, FIRST_TOKEN_ID).await;

    interact.set_fee_off_with_params(result, ALICE_ADDRESS, FIRST_TOKEN_ID).await;
}

#[tokio::test]
async fn test_multi_swap_pairs(){
    let mut interact= ContractInteract::new().await;

    interact.deploy().await;
    interact.set_pair_template_address().await;

    interact.create_pair_endpoint().await;

    let first_pair = interact.get_pair_with_params(
        FIRST_TOKEN_ID, 
        SECOND_TOKEN_ID
    ).await;

    let fee_percents = 100u64;
    let admins = MultiValueVec::from(vec![bech32::decode(BOB_ADDRESS), bech32::decode(ALICE_ADDRESS)]);

    interact.multi_pair_swap_with_params_fail(
        FIRST_TOKEN_ID, 
        0u64,
        1,
        first_pair.clone(), 
        SWAP_TOKENS_FIXED_INPUT_FUNC_NAME, 
        FIRST_TOKEN_ID, 
        100u128,
        ExpectError(4, "error signalled by smartcontract")
    ).await;

    interact.create_pair_endpoint_with_params(
        SECOND_TOKEN_ID,
        THIRD_TOKEN_ID, 
        fee_percents, 
        admins
    ).await;

    let second_pair = interact.get_pair_with_params(
        SECOND_TOKEN_ID, 
        THIRD_TOKEN_ID
    ).await;
        
    interact.issue_lp_token_with_params(
        LP_TOKEN_INITIAL_SUPPLY, 
        first_pair.clone(), 
        LP_TOKEN_NAME, 
        LP_TOKEN_TICKER
    ).await;

    interact.issue_lp_token_with_params(
        1000u128, 
        second_pair.clone(), 
        "LPTOKEN2", 
        "LPTT2"
    ).await;

    // interact.multi_pair_swap_with_params(
    //     FIRST_TOKEN_ID, 
    //     0u64, 
    //     1u128, 
    //     first_pair, 
    //     SWAP_TOKENS_FIXED_INPUT_FUNC_NAME, 
    //     SECOND_TOKEN_ID,
    //     1u128
    // ).await;

}