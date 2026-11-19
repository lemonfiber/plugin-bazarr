//! The manifest, as the core at the pinned commit reads it.

#[test]
fn the_pinned_core_refuses_nothing_in_the_manifest() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.toml"))
        .unwrap_or_default();
    let read: Result<lemonfiber_plugin::Manifest, _> = toml::from_str(&text);
    let refused: Vec<String> = match read {
        Ok(manifest) => lemonfiber_plugin::refusals(&manifest, &[])
            .into_iter()
            .map(|violation| format!("{}: {}", violation.location, violation.message))
            .collect(),
        Err(why) => vec![why.to_string()],
    };
    assert!(refused.is_empty(), "{refused:#?}");
}

#[test]
fn bazarr_knows_itself_by_a_name_and_not_by_its_container_id() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.toml"))
        .unwrap_or_default();
    let manifest: toml::Table = text.parse().unwrap_or_default();
    let named = manifest
        .get("service")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_table)
        .find(|service| service.get("id").and_then(toml::Value::as_str) == Some("bazarr"))
        .and_then(|service| service.get("container_hostname"))
        .and_then(toml::Value::as_str);
    assert_eq!(named, Some("bazarr"));
}
