use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::serde::Serialize;
use near_sdk::serde_json;
use near_sdk::{
    env, log, near_bindgen, require, AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault,
    Promise, PromiseError,
};
use serde_json::{json, Value as JsonValue};

pub mod constants;
pub mod utils;
pub use crate::constants::*;
pub use crate::utils::*;

pub type TimestampMs = u64;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    owner: AccountId,
    verified_complete_profiles: UnorderedMap<AccountId, TimestampMs>,
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
pub enum StorageKey {
    VerifiedCompleteProfiles,
}

#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum CheckType {
    CompleteSocialProfile,
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct CheckExternal {
    account_id: AccountId,
    check_type: CheckType,
    verified_at: TimestampMs,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner: Option<AccountId>) -> Self {
        Self {
            owner: owner.unwrap_or(env::signer_account_id()),
            verified_complete_profiles: UnorderedMap::new(StorageKey::VerifiedCompleteProfiles),
        }
    }

    pub fn has_complete_social_profile_check(&self, account_id: AccountId) -> bool {
        self.verified_complete_profiles.get(&account_id).is_some()
    }

    pub fn fetch_complete_social_profile_check(
        &self,
        account_id: AccountId,
    ) -> Option<CheckExternal> {
        self.verified_complete_profiles
            .get(&account_id)
            .map(|verified_at| CheckExternal {
                account_id,
                check_type: CheckType::CompleteSocialProfile,
                verified_at,
            })
    }

    pub fn remove_complete_social_profile_check(&mut self) {
        let account_id = env::predecessor_account_id();
        // update state
        if let Some(_verified_at) = self.verified_complete_profiles.remove(&account_id) {
            log!(format!(
                "Removing complete social profile check for '{}'",
                account_id
            ));
            // refund user for freed storage
            let initial_storage_usage = env::storage_usage();
            let storage_freed = initial_storage_usage - env::storage_usage();
            log!(format!("Storage freed: {} bytes", storage_freed));
            let cost_freed = env::storage_byte_cost()
                .checked_mul(storage_freed as u128)
                .unwrap();
            log!(format!("Cost freed: {} yoctoNEAR", cost_freed));
            if cost_freed.gt(&NearToken::from_near(0)) {
                log!(format!(
                    "Refunding {} yoctoNEAR to {}",
                    cost_freed, account_id
                ));
                Promise::new(account_id.clone()).transfer(cost_freed);
            }
        }
    }

    #[payable]
    pub fn verify_social_profile_completeness(&mut self) -> Promise {
        let account_id = env::predecessor_account_id();
        let attached_deposit = env::attached_deposit();
        let social_db: AccountId = if env::signer_account_id().to_string().ends_with(".near") {
            NEAR_SOCIAL_CONTRACT_ADDRESS_MAINNET.to_string()
        } else {
            NEAR_SOCIAL_CONTRACT_ADDRESS_TESTNET.to_string()
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
                    .verify_social_profile_completeness_callback(
                        account_id.clone(),
                        attached_deposit,
                    ),
            )
    }

    #[private]
    pub fn verify_social_profile_completeness_callback(
        &mut self,
        account_id: AccountId,
        attached_deposit: NearToken,
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

                    log!(
                        "Profile check for '{}': name={}, description={}, profile_image={}, background_image={}, linktree={}, tags={}",
                        account_id,
                        has_name,
                        has_description,
                        has_valid_profile_image,
                        has_valid_background_image,
                        has_linktree,
                        has_tags);

                    // Return true if all checks pass
                    let is_complete = has_name
                        && has_description
                        && has_valid_profile_image
                        && has_valid_background_image
                        && has_linktree
                        && has_tags;

                    if is_complete {
                        let initial_storage_usage = env::storage_usage();
                        // insert record
                        self.verified_complete_profiles
                            .insert(&account_id, &env::block_timestamp_ms());
                        // calculate storage cost
                        let required_deposit =
                            calculate_required_storage_deposit(initial_storage_usage);
                        // refund any unused deposit, or panic if not enough deposit attached
                        if attached_deposit.gt(&required_deposit) {
                            Promise::new(account_id.clone())
                                .transfer(attached_deposit.checked_sub(required_deposit).unwrap());
                        } else if attached_deposit.lt(&required_deposit) {
                            env::panic_str(&format!(
                                "Must attach {} yoctoNEAR to cover storage",
                                required_deposit
                            ));
                        }
                        true
                    } else {
                        false
                    }
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
