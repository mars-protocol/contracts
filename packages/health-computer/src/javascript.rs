use cosmwasm_schema::serde::{de::DeserializeOwned, Serialize};
use mars_rover_health_types::HealthResponse;
use wasm_bindgen::prelude::*;

use crate::HealthComputer;

#[wasm_bindgen]
pub fn compute_health_js(val: JsValue) -> JsValue {
    let c: HealthComputer = deserialize(val);
    let health = c.compute_health().unwrap();
    let health_response: HealthResponse = health.into();
    serialize(health_response)
}

pub fn serialize<T: Serialize>(val: T) -> JsValue {
    serde_wasm_bindgen::to_value(&val).unwrap()
}

pub fn deserialize<T: DeserializeOwned>(val: JsValue) -> T {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    serde_wasm_bindgen::from_value(val).unwrap()
}
