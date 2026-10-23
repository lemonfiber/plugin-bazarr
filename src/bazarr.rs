//! Bazarr, the subtitle finder, asked through its own API with the key lemonfiber wrote for
//! this adapter.

use std::sync::Arc;

use async_trait::async_trait;
use lemonfiber_ports::http::{Http, Method, Request};
use lemonfiber_ports::media::Kind;
use lemonfiber_ports::service::{Failure, Subtitles, Watched, Watching};
use serde::Deserialize;

use crate::credential::{spelled_as_a_key, Holding};
use crate::endpoint::{form_encoded, Endpoint};

/// What every failure is blamed on.
pub const SERVICE: &str = "bazarr";

const KEY_HEADER: &str = "X-API-KEY";

const SETTINGS: &str = "/api/system/settings";

const FRONT: &str = "/";

const CONTENT_TYPE: &str = "Content-Type";

const FORM: &str = "application/x-www-form-urlencoded";

const LONGEST_HOST: usize = 253;

#[derive(Deserialize)]
struct ArrSettings {
    #[serde(default)]
    ip: String,
    #[serde(default)]
    port: u16,
    #[serde(default)]
    apikey: String,
}

#[derive(Deserialize, Default)]
struct General {
    #[serde(default)]
    use_sonarr: bool,
    #[serde(default)]
    use_radarr: bool,
}

#[derive(Deserialize)]
struct Settings {
    #[serde(default)]
    general: General,
    #[serde(default)]
    sonarr: Option<ArrSettings>,
    #[serde(default)]
    radarr: Option<ArrSettings>,
}

/// A client for one Bazarr, presenting the key `held` supplies on every call.
pub struct Bazarr {
    endpoint: Endpoint,
    held: Arc<dyn Holding>,
}

impl Bazarr {
    /// The Bazarr reached at `base` over `http`, asked with the key `held` supplies.
    #[must_use]
    pub fn new(http: Arc<dyn Http>, base: impl Into<String>, held: Arc<dyn Holding>) -> Self {
        Self {
            endpoint: Endpoint::new(http, base, SERVICE),
            held,
        }
    }

    /// Whether Bazarr answers its front page, which it serves without a key.
    pub async fn answers(&self) -> bool {
        let request = Request {
            method: Method::Get,
            url: self.endpoint.url(FRONT),
            headers: Vec::new(),
            body: None,
            pinned: None,
        };
        self.endpoint
            .send(&request)
            .await
            .is_ok_and(|response| response.is_success())
    }

    fn settings(&self, method: Method, form: Option<String>) -> Result<Request, Failure> {
        let key = self.held.key().map_err(|why| {
            self.endpoint
                .refused(&format!("this adapter holds no key for Bazarr: {why}"))
        })?;
        let mut headers = vec![(KEY_HEADER.to_owned(), key)];
        if form.is_some() {
            headers.push((CONTENT_TYPE.to_owned(), FORM.to_owned()));
        }
        Ok(Request {
            method,
            url: self.endpoint.url(SETTINGS),
            headers,
            body: form,
            pinned: None,
        })
    }
}

impl Bazarr {
    async fn held(&self, which: Kind) -> Result<(bool, Option<ArrSettings>), Failure> {
        let response = self
            .endpoint
            .send(&self.settings(Method::Get, None)?)
            .await?;
        let held: Settings = self.endpoint.decode(
            &response,
            "the subtitle finder's settings could not be read",
        )?;
        Ok(match which {
            Kind::Tv => (held.general.use_sonarr, held.sonarr),
            Kind::Movies => (held.general.use_radarr, held.radarr),
        })
    }

    async fn holding(&self, watched: &Watched) -> Result<(), Failure> {
        let (enabled, arr) = self.held(watched.which).await?;
        let holds = enabled
            && arr.is_some_and(|arr| {
                arr.ip == watched.host && arr.port == watched.port && arr.apikey == watched.api_key
            });
        if holds {
            Ok(())
        } else {
            Err(self
                .endpoint
                .refused("Bazarr did not answer the write and does not hold what was written"))
        }
    }
}

#[async_trait]
impl Subtitles for Bazarr {
    async fn watching(&self, which: Kind) -> Result<Watching, Failure> {
        let (enabled, arr) = self.held(which).await?;
        Ok(arr.map_or(
            Watching {
                enabled,
                host: String::new(),
                port: 0,
                keyed: false,
            },
            |arr| Watching {
                enabled,
                host: arr.ip,
                port: arr.port,
                keyed: !arr.apikey.is_empty(),
            },
        ))
    }

    async fn watch(&self, watched: &Watched) -> Result<(), Failure> {
        if !a_host(&watched.host) || !spelled_as_a_key(&watched.api_key) {
            return Err(self
                .endpoint
                .refused("that is not a host and key Bazarr could be pointed at"));
        }
        let section = section(watched.which);
        let port = watched.port.to_string();
        let form = form_encoded(&[
            (&field("general", &format!("use_{section}")), "true"),
            (&field(section, "ip"), &watched.host),
            (&field(section, "port"), &port),
            (&field(section, "apikey"), &watched.api_key),
        ]);
        let request = self.settings(Method::Post, Some(form))?;
        if let Ok(written) = self.endpoint.send(&request).await {
            return self.endpoint.expect_success(&written);
        }
        self.holding(watched).await
    }
}

const fn section(kind: Kind) -> &'static str {
    match kind {
        Kind::Tv => "sonarr",
        Kind::Movies => "radarr",
    }
}

fn field(section: &str, name: &str) -> String {
    format!("settings-{section}-{name}")
}

fn a_host(host: &str) -> bool {
    (1..=LONGEST_HOST).contains(&host.len())
        && host
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || matches!(letter, '-' | '.' | ':'))
}

#[cfg(test)]
mod tests;
