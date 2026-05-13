use openplayer_shared::AppInfo;

pub fn app_info() -> AppInfo {
    AppInfo::skeleton(env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openplayer_shared::AppStage;

    #[test]
    fn reports_openplayer_skeleton_info() {
        let info = app_info();

        assert_eq!(info.name, "OpenPlayer");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.stage, AppStage::Skeleton);
    }
}
