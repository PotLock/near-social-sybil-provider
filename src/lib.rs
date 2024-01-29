use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::serde_json;
use near_sdk::{
    env, near_bindgen, AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault, Promise,
    PromiseError,
};
use serde_json::{json, Value as JsonValue};

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
        let social_db: AccountId = if env::signer_account_id().to_string().ends_with(".near") {
            "social.near"
        } else {
            "v1.social08.testnet"
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
                    .on_check_profile(account_id.clone()),
            )
    }

    #[private]
    pub fn on_check_profile(
        &self,
        account_id: AccountId,
        #[callback_result] call_result: Result<JsonValue, PromiseError>,
    ) -> bool {
        match call_result {
            Ok(JsonValue::Object(data)) => {
                if let Some(profile) = data
                    .get(account_id.as_str())
                    .and_then(|v| v.get("profile").and_then(|p| p.as_object()))
                {
                    // Check if 'name' exists and is valid
                    let has_name = profile
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map_or(false, |name| !name.is_empty());

                    // Check if 'description' exists and is valid
                    let has_description = profile
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map_or(false, |desc| !desc.is_empty());

                    // Check if 'image' exists and is valid
                    let has_valid_profile_image = profile
                        .get("image")
                        .map_or(false, |img| self.is_valid_image(img));

                    // Check if 'backgroundImage' exists and is valid
                    let has_valid_background_image = profile
                        .get("backgroundImage")
                        .map_or(false, |bg_img| self.is_valid_image(bg_img));

                    // Check if 'linktree' exists and is valid (at least one non-empty-string entry)
                    let has_linktree = profile.get("linktree").and_then(|l| l.as_object()).map_or(
                        false,
                        |linktree| {
                            linktree.iter().any(|(_, v)| {
                                if let Some(link) = v.as_str() {
                                    !link.is_empty()
                                } else {
                                    false
                                }
                            })
                        },
                    );

                    // Check if 'tags' exists and is valid (at least one non-empty-string entry)
                    let has_tags = profile
                        .get("tags")
                        .and_then(|t| t.as_object())
                        .map_or(false, |tags| !tags.is_empty());

                    // Return true if all checks pass
                    has_name
                        && has_description
                        && has_valid_profile_image
                        && has_valid_background_image
                        && has_linktree
                        && has_tags
                } else {
                    false
                }
            }
            Err(_) => false,
            _ => false,
        }
    }

    pub(crate) fn is_valid_image(&self, image: &serde_json::Value) -> bool {
        image
            .get("url")
            .and_then(|u| u.as_str())
            .map_or(false, |url| !url.is_empty())
            || image
                .get("ipfs_cid")
                .and_then(|cid| cid.as_str())
                .map_or(false, |cid| !cid.is_empty())
            || image
                .get("nft")
                .and_then(|nft| nft.as_object())
                .map_or(false, |nft| {
                    nft.get("contractId")
                        .and_then(|id| id.as_str())
                        .map_or(false, |id| !id.is_empty())
                        && nft
                            .get("tokenId")
                            .and_then(|token| token.as_str())
                            .map_or(false, |token| !token.is_empty())
                })
    }
}
