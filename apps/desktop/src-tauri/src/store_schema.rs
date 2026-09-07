use redb::{Database, ReadableDatabase, TableDefinition, TableError};

pub(crate) fn ensure_string_tables(
    database: &Database,
    tables: &[TableDefinition<&str, &str>],
) -> Result<bool, String> {
    let read = database.begin_read().map_err(|error| error.to_string())?;
    let mut missing = false;
    for table in tables {
        match read.open_table(*table) {
            Ok(_) => {}
            Err(TableError::TableDoesNotExist(_)) => missing = true,
            Err(error) => return Err(error.to_string()),
        }
    }
    drop(read);
    if !missing {
        return Ok(false);
    }
    let write = database.begin_write().map_err(|error| error.to_string())?;
    for table in tables {
        write
            .open_table(*table)
            .map_err(|error| error.to_string())?;
    }
    write.commit().map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use redb::backends::InMemoryBackend;

    #[test]
    fn initializes_only_missing_tables_and_preserves_existing_values() {
        let db = Database::builder()
            .create_with_backend(InMemoryBackend::new())
            .unwrap();
        let first = TableDefinition::new("first");
        let second = TableDefinition::new("second");
        assert!(ensure_string_tables(&db, &[first]).unwrap());
        let write = db.begin_write().unwrap();
        write
            .open_table(first)
            .unwrap()
            .insert("key", "value")
            .unwrap();
        write.commit().unwrap();
        assert!(!ensure_string_tables(&db, &[first]).unwrap());
        assert!(ensure_string_tables(&db, &[first, second]).unwrap());
        assert!(!ensure_string_tables(&db, &[first, second]).unwrap());
        assert_eq!(
            db.begin_read()
                .unwrap()
                .open_table(first)
                .unwrap()
                .get("key")
                .unwrap()
                .unwrap()
                .value(),
            "value"
        );
    }
}
