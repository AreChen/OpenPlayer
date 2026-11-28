use std::{env, error::Error, path::Path};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde_json::{Value, json};

const HISTORY: TableDefinition<&str, &str> = TableDefinition::new("history_by_path");
const PLAYBACK: TableDefinition<&str, &str> = TableDefinition::new("playback_settings");
const PREFERENCES: TableDefinition<&str, &str> = TableDefinition::new("settings_kv");
const FAVORITES: &str = "mediaFavoriteFiles";

fn main() -> Result<(), Box<dyn Error>> {
    let [_, action, storage, media] = env::args()
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "expected package-profile <read|seed> STORAGE_DIRECTORY MEDIA_PATH")?;
    let storage = Path::new(&storage);
    let playback = Database::open(storage.join("playback-history.redb"))?;
    if action == "seed" {
        seed(&playback, storage, &media)?;
    } else if action != "read" {
        return Err("expected read or seed".into());
    }

    let transaction = playback.begin_read()?;
    let table = transaction.open_table(PLAYBACK)?;
    let settings = read_json(table.get("global")?.map(|value| value.value().to_owned()))?;
    let table = transaction.open_table(HISTORY)?;
    let mut history = Vec::new();
    for row in table.iter()? {
        let (_, item) = row?;
        history.push(serde_json::from_str::<Value>(item.value())?);
    }
    let preference_path = storage.join("openplayer-settings.redb");
    let favorites = if preference_path.is_file() {
        let preferences = Database::open(preference_path)?;
        let transaction = preferences.begin_read()?;
        let table = transaction.open_table(PREFERENCES)?;
        read_json(table.get(FAVORITES)?.map(|value| value.value().to_owned()))?
    } else {
        Value::Null
    };
    println!(
        "{}",
        json!({ "settings": settings, "history": history, "favorites": favorites })
    );
    Ok(())
}

fn read_json(encoded: Option<String>) -> serde_json::Result<Value> {
    encoded
        .as_deref()
        .map_or(Ok(Value::Null), serde_json::from_str)
}

fn seed(playback: &Database, storage: &Path, media: &str) -> Result<(), Box<dyn Error>> {
    let transaction = playback.begin_write()?;
    {
        let mut table = transaction.open_table(PLAYBACK)?;
        let mut settings = read_json(table.get("global")?.map(|value| value.value().to_owned()))?;
        if settings.is_null() {
            settings = json!({
                "loopMode": "off",
                "hwdecMode": "hardware",
                "playbackSpeed": 1.0,
                "videoFill": false,
                "timeDisplayMode": "timecode"
            });
        }
        settings["volume"] = json!(31.0);
        table.insert("global", serde_json::to_string(&settings)?.as_str())?;
    }
    transaction.commit()?;

    {
        let preferences = Database::create(storage.join("openplayer-settings.redb"))?;
        let transaction = preferences.begin_write()?;
        {
            let mut table = transaction.open_table(PREFERENCES)?;
            table.insert(FAVORITES, serde_json::to_string(&[media])?.as_str())?;
        }
        transaction.commit()?;
    }
    Ok(())
}
