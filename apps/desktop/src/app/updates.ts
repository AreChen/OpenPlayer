import type { AppStrings } from "../i18n";
import type { UpdateState } from "./types";

export function canCheckForUpdates({ status }: UpdateState) {
  return status === "idle" || status === "current" || status === "unsupported" || status === "available"
    || status === "checkFailed" || status === "downloadFailed" || status === "installFailed";
}

export function updateActionLabel(state: UpdateState, t: AppStrings) {
  const about = t.settings.about;
  switch (state.status) {
    case "available":
    case "downloadFailed":
    case "installFailed":
      return about.downloadUpdate;
    case "ready":
    case "installerOpened":
      return state.update.package === "dmg" ? about.openInstaller : about.installUpdate;
    case "installed":
    case "restartFailed":
      return about.restartUpdate;
    default:
      return null;
  }
}

export function updateStatusText(state: UpdateState, t: AppStrings) {
  const about = t.settings.about;
  switch (state.status) {
    case "idle":
      return about.updateIdle;
    case "checking":
      return about.checkingUpdates;
    case "current":
      return about.upToDate;
    case "unsupported":
      return about.updateUnsupported;
    case "available":
      return about.updateAvailable(state.update.version);
    case "downloading":
      return about.downloadingUpdate;
    case "ready":
      return about.updateReady(state.update.version);
    case "installing":
      return about.installingUpdate;
    case "installed":
      return about.updateInstalled(state.update.version);
    case "installerOpened":
      return about.updateInstallerOpened;
    case "checkFailed":
      return about.updateCheckFailed(state.message);
    case "downloadFailed":
      return about.updateDownloadFailed(state.message);
    case "installFailed":
      return about.updateInstallFailed(state.message);
    case "restartFailed":
      return about.updateRestartFailed(state.message);
  }
}
