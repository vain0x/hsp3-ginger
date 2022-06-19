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
 * 拡張機能がロードされたときに呼ばれる。
 */
export const activate = (context: ExtensionContext) => {
    const distDir = path.join(context.extensionPath, "dist")

    const configProvider = new MyConfigurationProvider(distDir)
    context.subscriptions.push(
        debug.registerDebugConfigurationProvider(
            HSP3_LANG_ID,
            configProvider
        ))
}
