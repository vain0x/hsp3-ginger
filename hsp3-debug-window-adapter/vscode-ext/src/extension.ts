// 拡張機能のエントリーポイント

import * as path from "path"
import { ExtensionContext, debug, commands, window } from "vscode"
import { adapterExecutableCommand } from "./ext_command_adapter_executable"
import { createHsptmpCommand } from "./ext_command_create_hsptmp"
import { MyConfigurationProvider } from "./ext_config_provider"
import { HSP3_LANG_ID } from "./ext_constants"

/**
 * デバッガーのディレクトリへのパス。
 *
 * FIXME: 名前が適切でない。
 */
const getExtensionRoot = (extensionPath: string) =>
    path.join(extensionPath, "out")

/**
 * 拡張機能がロードされたときに呼ばれる。
 */
export const activate = (context: ExtensionContext) => {
    const extensionRoot = getExtensionRoot(context.extensionPath)

    const configProvider = new MyConfigurationProvider(extensionRoot)
    context.subscriptions.push(
        debug.registerDebugConfigurationProvider(
            HSP3_LANG_ID,
            configProvider
        ))

    context.subscriptions.push(
        commands.registerCommand(
            "hsp3-debug-window-adapter.adapterExecutableCommand",
            adapterExecutableCommand(extensionRoot),
        ))

    context.subscriptions.push(
        commands.registerCommand(
            "hsp3-debug-window-adapter.createHsptmp",
            createHsptmpCommand,
        ))
}

/**
 * 拡張機能が停止されるときに呼ばれる。
 */
export const deactivate = () => {
    // Pass.
}
