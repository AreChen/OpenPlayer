mod media_paths;
mod playback_store;
mod player;
#[cfg(windows)]
mod shell_preview;
mod store_schema;
#[cfg(unix)]
mod unix_package;
mod updater;

pub use player::run;
