// 拡張機能のエントリーポイント

import * as path from "path"
import { ExtensionContext, debug } from "vscode"
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
 * 拡張機能がロードされたときに呼ばれる。
 */
export const activate = (context: ExtensionContext) => {
    const distDir = path.join(context.extensionPath, "dist")

    context.subscriptions.push(
        debug.registerDebugConfigurationProvider(
            HSP3_LANG_ID,
            new MyConfigurationProvider(distDir)
        ))
}
