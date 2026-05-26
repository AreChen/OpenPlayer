use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

pub(in crate::media_paths) fn sort_media_paths(paths: &mut [PathBuf]) {
    paths.sort_by(|left, right| {
        natural_path_cmp(left, right).then_with(|| {
            left.to_string_lossy()
                .to_ascii_lowercase()
                .cmp(&right.to_string_lossy().to_ascii_lowercase())
        })
    });
}

fn natural_path_cmp(left: &Path, right: &Path) -> Ordering {
    let left_name = path_file_name(left);
    let right_name = path_file_name(right);
    natural_str_cmp(&left_name, &right_name)
}

fn path_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn natural_str_cmp(left: &str, right: &str) -> Ordering {
    let left_tokens = natural_tokens(left);
    let right_tokens = natural_tokens(right);
    for (left_token, right_token) in left_tokens.iter().zip(right_tokens.iter()) {
        let ordering = left_token.cmp(right_token);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left_tokens.len().cmp(&right_tokens.len())
}

#[derive(Debug, Eq, PartialEq)]
enum NaturalToken {
    Text(String),
    Number { value: String, raw_len: usize },
}

impl Ord for NaturalToken {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Text(left), Self::Text(right)) => left.cmp(right),
            (
                Self::Number {
                    value: left,
                    raw_len: left_len,
                },
                Self::Number {
                    value: right,
                    raw_len: right_len,
                },
            ) => left
                .len()
                .cmp(&right.len())
                .then_with(|| left.cmp(right))
                .then_with(|| left_len.cmp(right_len)),
            (Self::Number { .. }, Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Number { .. }) => Ordering::Greater,
        }
    }
}

impl PartialOrd for NaturalToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn natural_tokens(text: &str) -> Vec<NaturalToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_is_digit: Option<bool> = None;

    for character in text.chars() {
        let is_digit = character.is_ascii_digit();
        if current_is_digit.is_some_and(|digit| digit != is_digit) {
            tokens.push(natural_token(&current, current_is_digit.unwrap_or(false)));
            current.clear();
        }
        current_is_digit = Some(is_digit);
        current.push(character);
    }

    if !current.is_empty() {
        tokens.push(natural_token(&current, current_is_digit.unwrap_or(false)));
    }

    tokens
}

fn natural_token(text: &str, is_digit: bool) -> NaturalToken {
    if is_digit {
        let trimmed = text.trim_start_matches('0');
        NaturalToken::Number {
            value: if trimmed.is_empty() {
                "0".to_string()
            } else {
                trimmed.to_string()
            },
            raw_len: text.len(),
        }
    } else {
        NaturalToken::Text(text.to_string())
    }
}
