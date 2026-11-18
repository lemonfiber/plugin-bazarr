//! What this adapter says it is, and the kit that serves its contract.

use std::sync::Arc;

use lemonfiber_adapter_kit::{Kit, Readiness};
use lemonfiber_contract::adapter::{About, Release};
use lemonfiber_contract::capabilities::subtitles;

use crate::bazarr::Bazarr;

/// Where Bazarr answers, by its service id and the port it listens on.
pub const ADDRESS: &str = "http://bazarr:6767";

/// The part of `upstream.json` holding the key Bazarr answers its API to.
pub const API_KEY: &str = "api_key";

/// The upstream this adapter fronts, as its project names itself.
pub const UPSTREAM: &str = "Bazarr";

include!(concat!(env!("OUT_DIR"), "/fronted.rs"));

/// What this adapter says it is: the contract it speaks and the release it supports.
#[must_use]
pub fn about() -> About {
    About {
        speaks: vec![lemonfiber_contract::spoken(
            subtitles::fetch::CAPABILITY,
            subtitles::fetch::MAJOR,
        )],
        upstream: UPSTREAM.to_owned(),
        releases: vec![Release {
            version: RELEASE.to_owned(),
            digest: DIGEST.to_owned(),
        }],
    }
}

/// The contract, served from `bazarr` under `key`, ready while Bazarr answers.
#[must_use]
pub fn kit(key: String, bazarr: Arc<Bazarr>) -> Kit {
    let asked = Arc::clone(&bazarr);
    let ready: Readiness = Arc::new(move || {
        let asked = Arc::clone(&asked);
        Box::pin(async move { asked.answers().await })
    });
    Kit::new(key, about())
        .ready_when(ready)
        .serving(subtitles::fetch::served(bazarr))
}

#[cfg(test)]
mod tests;
