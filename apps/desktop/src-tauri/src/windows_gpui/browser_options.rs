use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct BrowserOptions {
    pub(super) sort: Sort,
    pub(super) category: Category,
    pub(super) only_media: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Category {
    #[default]
    All,
    Video,
    Audio,
    Folders,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Sort {
    pub(super) by: SortBy,
    pub(super) descending: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum SortBy {
    #[default]
    Name,
    Type,
    Modified,
    Size,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_options_default_to_all_entries_and_restore_saved_choices() {
        let defaults: BrowserOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(defaults, BrowserOptions::default());
        assert!(!defaults.only_media);

        let choices = BrowserOptions {
            sort: Sort {
                by: SortBy::Modified,
                descending: true,
            },
            category: Category::Video,
            only_media: true,
        };
        let encoded = serde_json::to_string(&choices).unwrap();
        assert_eq!(
            serde_json::from_str::<BrowserOptions>(&encoded).unwrap(),
            choices
        );
    }
}
