use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, Promise, near_bindgen, AccountId, BorshStorageKey, Gas, PromiseError, PanicOnDefault, NearToken};
use near_sdk::serde_json;
use serde_json::{Value as JsonValue, json};


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct ProviderContract {
    owner_id: AccountId,
}

#[near_bindgen]
impl ProviderContract {

    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            owner_id: env::signer_account_id(),
        }
    }

    pub fn check_profile(&self, account_id: AccountId) -> Promise {

        let social_db: AccountId = if env::signer_account_id().to_string().ends_with("testnet") {
            "v1.social08.testnet"
        } else {
            "social.near"
        }
        .parse()
        .unwrap();

        let key = format!("{}/profile/**", account_id);
    
        Promise::new(social_db)
        .function_call(
            "get".to_string(),
            serde_json::to_vec(&serde_json::json!({"keys": [key]})).unwrap(),
            NearToken::from_near(0),
            Gas::from_tgas(50),
        )
        .then(
            Self::ext(env::current_account_id())
                //.with_static_gas(env::prepaid_gas().saturating_div(3))
                .on_check_profile()
        )
    }
    
    #[private]
    pub fn on_check_profile(&self, #[callback_result] call_result: Result<JsonValue, PromiseError>) -> bool {
        match call_result {
            Ok(data) => {
            
                // Check if the JSON object is not empty
                !data.as_object().map_or(true, |obj| obj.is_empty())
            }
            Err(_) => false,
        }
    }  
  
}