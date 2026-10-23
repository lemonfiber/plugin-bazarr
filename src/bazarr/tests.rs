use std::sync::Arc;

use lemonfiber_adapter_kit::credential::Holding;
use lemonfiber_fixtures::http::{Answer, Fake};
use lemonfiber_ports::http::Method;
use lemonfiber_ports::media::Kind;
use lemonfiber_ports::service::{Failure, Subtitles, Watched, Watching};

use super::Bazarr;

const BASE: &str = "http://127.0.0.1:6767";

const KEY: &str = "the-key";

const SETTINGS: &str = r#"{
    "general": {"use_sonarr": true, "use_radarr": false},
    "sonarr": {"ip": "sonarr", "port": 8989, "apikey": "abc", "base_url": "/"},
    "radarr": {"ip": "", "port": 7878, "apikey": ""}
}"#;

fn client(replies: Vec<(u16, &'static str)>) -> (Bazarr, Arc<Fake>) {
    let http = Fake::scripted(replies);
    (
        Bazarr::new(http.clone(), BASE, Arc::new(KEY.to_owned())),
        http,
    )
}

fn a_curator() -> Watched {
    Watched {
        which: Kind::Movies,
        host: "radarr".to_owned(),
        port: 7878,
        api_key: ('a'..='p').collect(),
    }
}

struct Nothing;

impl Holding for Nothing {
    fn key(&self) -> Result<String, String> {
        Err("upstream.json could not be read: not found".to_owned())
    }
}

#[tokio::test]
async fn what_it_holds_for_each_curator_is_read_from_its_settings() {
    let (bazarr, http) = client(vec![(200, SETTINGS), (200, SETTINGS)]);
    assert_eq!(
        bazarr.watching(Kind::Tv).await.ok(),
        Some(Watching {
            enabled: true,
            host: "sonarr".to_owned(),
            port: 8989,
            keyed: true,
        })
    );
    assert_eq!(
        bazarr.watching(Kind::Movies).await.ok(),
        Some(Watching {
            enabled: false,
            host: String::new(),
            port: 7878,
            keyed: false,
        })
    );
    let asked = http.requests();
    assert!(asked.iter().all(|one| one.method == Method::Get
        && one.url == format!("{BASE}/api/system/settings")
        && one
            .headers
            .contains(&("X-API-KEY".to_owned(), KEY.to_owned()))));
}

#[tokio::test]
async fn a_curator_it_holds_no_section_for_is_watched_nowhere() {
    let (bazarr, _) = client(vec![(200, r#"{"general":{}}"#)]);
    assert_eq!(
        bazarr.watching(Kind::Movies).await.ok(),
        Some(Watching {
            enabled: false,
            host: String::new(),
            port: 0,
            keyed: false,
        })
    );
}

#[tokio::test]
async fn settings_that_will_not_read_are_a_failure() {
    assert!(matches!(
        client(vec![(200, "not json")]).0.watching(Kind::Tv).await,
        Err(Failure::Refused { .. })
    ));
    assert!(matches!(
        client(vec![(401, "")]).0.watching(Kind::Tv).await,
        Err(Failure::Unauthorised { .. })
    ));
}

#[tokio::test]
async fn a_curator_is_watched_with_its_switch_and_its_address_in_one_form() {
    let (bazarr, http) = client(vec![(204, "")]);
    assert_eq!(bazarr.watch(&a_curator()).await.ok(), Some(()));
    let sent = http.requests();
    let one = sent.first();
    assert_eq!(
        one.map(|one| (one.method, one.url.as_str(), one.body.as_deref())),
        Some((
            Method::Post,
            format!("{BASE}/api/system/settings").as_str(),
            Some("settings-general-use_radarr=true&settings-radarr-ip=radarr&settings-radarr-port=7878&settings-radarr-apikey=abcdefghijklmnop")
        ))
    );
    assert!(one.is_some_and(|one| one.headers.contains(&(
        "Content-Type".to_owned(),
        "application/x-www-form-urlencoded".to_owned()
    ))));
}

const HOLDING_IT: &str = r#"{
    "general": {"use_radarr": true},
    "radarr": {"ip": "radarr", "port": 7878, "apikey": "abcdefghijklmnop"}
}"#;

#[tokio::test]
async fn a_write_bazarr_withholds_its_answer_to_is_confirmed_by_reading_it_back() {
    let http = Fake::in_turn(vec![Answer::Silent, Answer::reply(200, HOLDING_IT)]);
    let bazarr = Bazarr::new(http.clone(), BASE, Arc::new(KEY.to_owned()));
    assert_eq!(bazarr.watch(&a_curator()).await.ok(), Some(()));
    assert_eq!(
        http.requests()
            .iter()
            .map(|one| one.method)
            .collect::<Vec<_>>(),
        [Method::Post, Method::Get]
    );
    for read in [
        Answer::reply(200, SETTINGS),
        Answer::reply(200, r#"{"general":{"use_radarr":true}}"#),
        Answer::Silent,
    ] {
        let http = Fake::in_turn(vec![Answer::Silent, read]);
        let bazarr = Bazarr::new(http, BASE, Arc::new(KEY.to_owned()));
        assert!(bazarr.watch(&a_curator()).await.is_err());
    }
}

#[tokio::test]
async fn a_write_bazarr_refuses_is_a_failure() {
    let (bazarr, _) = client(vec![(500, "boom")]);
    assert!(bazarr.watch(&a_curator()).await.is_err());
}

#[tokio::test]
async fn a_host_or_key_bazarr_could_not_be_pointed_at_is_refused_before_anything_is_asked() {
    for watched in [
        Watched {
            host: String::new(),
            ..a_curator()
        },
        Watched {
            host: "radarr&settings-general-use_sonarr=false".to_owned(),
            ..a_curator()
        },
        Watched {
            host: "r".repeat(254),
            ..a_curator()
        },
        Watched {
            host: "radarr:7878".to_owned(),
            ..a_curator()
        },
        Watched {
            host: "fd00::7".to_owned(),
            ..a_curator()
        },
        Watched {
            host: "[radarr]".to_owned(),
            ..a_curator()
        },
        Watched {
            host: "-radarr".to_owned(),
            ..a_curator()
        },
        Watched {
            api_key: "a key".to_owned(),
            ..a_curator()
        },
    ] {
        let (bazarr, http) = client(vec![(204, "")]);
        assert!(matches!(
            bazarr.watch(&watched).await,
            Err(Failure::Refused { .. })
        ));
        assert!(http.requests().is_empty());
    }
    for host in ["10.0.0.5", "[fd00::7]", "radarr.lan"] {
        let (bazarr, _) = client(vec![(204, "")]);
        let watched = Watched {
            host: host.to_owned(),
            which: Kind::Tv,
            ..a_curator()
        };
        assert!(bazarr.watch(&watched).await.is_ok(), "{host}");
    }
}

#[tokio::test]
async fn without_a_key_nothing_is_asked_and_the_refusal_says_why() {
    let http = Fake::silent();
    let bazarr = Bazarr::new(http.clone(), BASE, Arc::new(Nothing));
    assert!(matches!(
        bazarr.watching(Kind::Tv).await,
        Err(Failure::Refused { detail, .. })
            if detail == "this adapter holds no key for Bazarr: upstream.json could not be read: not found"
    ));
    assert!(bazarr.watch(&a_curator()).await.is_err());
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn it_answers_while_its_front_page_answers() {
    let up = Fake::always(Answer::reply(200, "<html/>"));
    assert!(
        Bazarr::new(up.clone(), BASE, Arc::new(Nothing))
            .answers()
            .await
    );
    assert!(up
        .requests()
        .iter()
        .all(|one| one.url == format!("{BASE}/")));
    for down in [Fake::silent(), Fake::always(Answer::reply(503, ""))] {
        assert!(!Bazarr::new(down, BASE, Arc::new(Nothing)).answers().await);
    }
}
