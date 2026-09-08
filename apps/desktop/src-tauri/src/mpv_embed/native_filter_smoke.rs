//! Opt-in developer smoke fixture. Not an IPC command or a public plugin API.
use super::*;

pub(crate) async fn command(app: AppHandle, name: &str, args: Vec<String>) -> Result<(), String> {
    let name = name.to_owned();
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            let args = args.iter().map(String::as_str).collect::<Vec<_>>();
            player
                .mpv
                .command(&name, &args)
                .map_err(|error| error.to_string())
        })
    })
    .await
}

pub(crate) async fn attach(app: AppHandle, script: String) -> Result<(), String> {
    let path = PathBuf::from(&script);
    if !path.is_absolute() || !path.is_file() || path.extension().is_none_or(|ext| ext != "vpy") {
        return Err("native filter smoke requires an absolute local .vpy fixture".into());
    }
    let filter = format!(
        "@native-smoke:vapoursynth=file=%{}%{}:concurrent-frames=1",
        script.len(),
        script
    );
    command(app, "vf", vec!["add".into(), filter]).await
}
