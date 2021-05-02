// 拡張機能のエントリーポイント

import * as path from "path"
import { ExtensionContext, debug, commands, window } from "vscode"
import { createHsptmp } from "./ext_command_create_hsptmp"
import { MyConfigurationProvider } from "./ext_config_provider"
import { HSP3_LANG_ID } from "./ext_constants"

/**
 * 想定内のエラーを表す。
 */
export class DomainError extends Error {
    public constructor(userFriendlyMessage: string) {
        super(userFriendlyMessage)
    }

    public toString() {
        return this.message
    }
}

/**
 * 非同期処理の例外をキャッチしてエラーメッセージを表示する。
 */
export const withNotify = <T>(body: () => Promise<T>) =>
    () => body().catch(err => {
        const message = err instanceof Error ? err.toString() : String(err)
        window.showErrorMessage(message)
        return null
    })

/**
 * outディレクトリへのパス。
 */
const getOutDir = (extensionPath: string) =>
    path.join(extensionPath, "out")

/**
 * 拡張機能がロードされたときに呼ばれる。
 */
export const activate = (context: ExtensionContext) => {
    const outDir = getOutDir(context.extensionPath)

    const configProvider = new MyConfigurationProvider(outDir)
    context.subscriptions.push(
        debug.registerDebugConfigurationProvider(
            HSP3_LANG_ID,
            configProvider
        ))

    context.subscriptions.push(
        commands.registerCommand(
            "hsp3-debug-window-adapter.createHsptmp",
            withNotify(createHsptmp),
        ))
}

/**
 * 拡張機能が停止されるときに呼ばれる。
 */
export const deactivate = () => {
    // Pass.
}
