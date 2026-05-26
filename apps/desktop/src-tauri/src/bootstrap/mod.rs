#[cfg(feature = "mpv-embed")]
mod embedded;
#[cfg(not(feature = "mpv-embed"))]
mod fallback;

#[cfg(feature = "mpv-embed")]
pub use embedded::run;
#[cfg(not(feature = "mpv-embed"))]
pub use fallback::run;
