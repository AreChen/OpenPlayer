import { pluginWorkerAudioApiSource } from "./apiSections/audio";
import { pluginWorkerArtifactsApiSource } from "./apiSections/artifacts";
import { pluginWorkerCaptureApiSource } from "./apiSections/capture";
import { pluginWorkerCommandsApiSource } from "./apiSections/commands";
import { pluginWorkerFilesystemApiSource } from "./apiSections/filesystem";
import { pluginWorkerLogApiSource } from "./apiSections/log";
import { pluginWorkerMediaApiSource } from "./apiSections/media";
import { pluginWorkerMpvApiSource } from "./apiSections/mpv";
import { pluginWorkerNetworkApiSource } from "./apiSections/network";
import { pluginWorkerPluginApiSource } from "./apiSections/plugin";
import { pluginWorkerPlayerApiSource } from "./apiSections/player";
import { pluginWorkerPlaylistApiSource } from "./apiSections/playlist";
import { pluginWorkerStorageApiSource } from "./apiSections/storage";
import { pluginWorkerSubtitleApiSource } from "./apiSections/subtitle";
import { pluginWorkerTasksApiSource } from "./apiSections/tasks";
import { pluginWorkerUiApiSource } from "./apiSections/ui";

export function pluginWorkerApiObjectMembersSource() {
  return [
    pluginWorkerCommandsApiSource(),
    pluginWorkerPluginApiSource(),
    pluginWorkerLogApiSource(),
    pluginWorkerTasksApiSource(),
    pluginWorkerArtifactsApiSource(),
    pluginWorkerMediaApiSource(),
    pluginWorkerPlayerApiSource(),
    pluginWorkerAudioApiSource(),
    pluginWorkerMpvApiSource(),
    pluginWorkerCaptureApiSource(),
    pluginWorkerStorageApiSource(),
    pluginWorkerNetworkApiSource(),
    pluginWorkerUiApiSource(),
    pluginWorkerPlaylistApiSource(),
    pluginWorkerFilesystemApiSource(),
    pluginWorkerSubtitleApiSource(),
  ].join(",\n");
}
