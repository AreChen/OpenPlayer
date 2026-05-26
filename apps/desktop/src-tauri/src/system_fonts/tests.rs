use super::*;

#[test]
fn extracts_font_family_names_from_windows_registry_labels() {
    assert_eq!(
        registry_font_name_to_family("Arial Bold (TrueType)").as_deref(),
        Some("Arial")
    );
    assert_eq!(
        registry_font_name_to_family("@Microsoft YaHei UI (TrueType)").as_deref(),
        Some("Microsoft YaHei UI")
    );
}

#[test]
fn normalizes_font_family_list_case_insensitively() {
    assert_eq!(
        normalize_font_family_list(vec![
            "Arial".to_string(),
            "arial".to_string(),
            "Segoe UI".to_string(),
        ]),
        vec!["Arial".to_string(), "Segoe UI".to_string()]
    );
    assert!(normalize_font_family_list(Vec::new()).contains(&"sans-serif".to_string()));
}
