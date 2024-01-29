# NEAR Social Sybil Provider

## Purpose

Provides a Sybil-resistance service by verifying various aspects of a user's NEAR Social profile, e.g. profile completeness (more checks to come soon)

## Contract Structure

### General Types

```rs
pub type TimestampMs = u64;
```

### Contract

```rs
pub struct Contract {
    owner: AccountId,
    verified_complete_profiles: UnorderedMap<AccountId, TimestampMs>,
}
```

### Checks

```rs
pub enum CheckType {
    CompleteSocialProfile,
}

pub struct CheckExternal {
    account_id: AccountId,
    check_type: CheckType,
    verified_at: TimestampMs,
}
```

## Methods

### Write Methods

```rs
// INIT

pub fn new(owner: Option<AccountId>) -> Self


// CHECKS

#[payable]
pub fn verify_social_profile_completeness(&mut self) -> bool // Uses approx. 190 bytes of storage. returns True if social profile was successfully verified as complete.

pub fn remove_complete_social_profile_check(&mut self)
```

### Read Methods

```rs
// CHECKS

pub fn has_complete_social_profile_check(&self, account_id: AccountId) -> bool

pub fn fetch_complete_social_profile_check(
    &self,
    account_id: AccountId,
) -> Option<CheckExternal>
```