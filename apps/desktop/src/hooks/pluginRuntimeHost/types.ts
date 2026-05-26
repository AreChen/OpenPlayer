export type PluginRuntimeCommandHandler = (
  command: string,
  args: unknown,
  permissions: Set<string>,
  pluginId: string,
) => Promise<unknown>;
