//! What this adapter says it is, and the kit that serves its contract.

use std::sync::Arc;

use lemonfiber_adapter_kit::{Kit, Readiness};
use lemonfiber_contract::adapter::{About, Release};
use lemonfiber_contract::capabilities::subtitles;

use crate::bazarr::Bazarr;

/// The upstream this adapter fronts, as its project names itself.
pub const UPSTREAM: &str = "Bazarr";

/// The Bazarr release this adapter's recordings were taken from.
pub const RELEASE: &str = "v1.6.2-ls367";

/// The digest of the image that release's recordings were taken from.
pub const DIGEST: &str = "sha256:c1673816321156b904af9b7e6415ea547ae9622e4acc375358f1e1eb9ed1e904";

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
