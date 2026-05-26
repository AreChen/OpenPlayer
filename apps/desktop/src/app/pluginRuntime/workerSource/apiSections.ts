import { pluginWorkerCaptureApiSource } from "./apiSections/capture";
import { pluginWorkerCommandsApiSource } from "./apiSections/commands";
import { pluginWorkerFilesystemApiSource } from "./apiSections/filesystem";
import { pluginWorkerMediaApiSource } from "./apiSections/media";
import { pluginWorkerNetworkApiSource } from "./apiSections/network";
import { pluginWorkerPlayerApiSource } from "./apiSections/player";
import { pluginWorkerPlaylistApiSource } from "./apiSections/playlist";
import { pluginWorkerStorageApiSource } from "./apiSections/storage";
import { pluginWorkerSubtitleApiSource } from "./apiSections/subtitle";
import { pluginWorkerUiApiSource } from "./apiSections/ui";

export function pluginWorkerApiObjectMembersSource() {
  return [
    pluginWorkerCommandsApiSource(),
    pluginWorkerMediaApiSource(),
    pluginWorkerPlayerApiSource(),
    pluginWorkerCaptureApiSource(),
    pluginWorkerStorageApiSource(),
    pluginWorkerNetworkApiSource(),
    pluginWorkerUiApiSource(),
    pluginWorkerPlaylistApiSource(),
    pluginWorkerFilesystemApiSource(),
    pluginWorkerSubtitleApiSource(),
  ].join(",\n");
}
