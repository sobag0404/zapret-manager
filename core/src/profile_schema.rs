use crate::profiles::{Profile, Strategy};
use schemars::{schema::RootSchema, schema_for};

pub fn profile_schema() -> RootSchema {
    schema_for!(Profile)
}

pub fn strategy_schema() -> RootSchema {
    schema_for!(Strategy)
}
