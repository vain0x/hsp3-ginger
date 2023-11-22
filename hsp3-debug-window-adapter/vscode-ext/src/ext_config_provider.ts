import * as fsP from "fs/promises"
import { CancellationToken, DebugConfiguration, DebugConfigurationProvider, WorkspaceFolder, window, ProviderResult } from "vscode"
import { selectHsp3Root } from "./ext_command_select_hsp3_root"
import { createHsptmp } from "./ext_command_create_hsptmp"
import { HSP3_LANG_ID } from "./ext_constants"
import { decode } from "iconv-lite"

/**
 * デバッガーの設定を構成する。
 *
 * ここでの設定が "launch" リスエストに渡される。
 */
const doResolveDebugConfiguration = async (config: DebugConfiguration, distDir: string) => {
    // launch.json ファイルがないか、デバッグ構成がないとき
    if (!config.type && !config.request && !config.name) {
        const editor = window.activeTextEditor
        if (editor && editor.document.languageId === HSP3_LANG_ID) {
            config.type = HSP3_LANG_ID
            config.name = "Run"
            config.request = "launch"
        }
    }

    if (!config.type || !config.request) {
        window.showWarningMessage("HSP3 でデバッグ可能なファイルではありません。")
        return null
    }

    let utf8Support: string | undefined = config.utf8Support || "auto"
    if (utf8Support === "auto") {
        let text: string | undefined
        if (config.program) {
            const contents = (await fsP.readFile(config.program))

            try {
                text = decode(contents, "utf-8")
            } catch (err) {
                // Pass.
            }
        } else {
            text = window.activeTextEditor?.document?.getText()
        }

        const utf8 = text != null && (
            text.includes("#include \"hsp3utf.as\"")
            || text.includes("#include \"hsp3_64.as\"")
        )
        utf8Support = utf8 ? "enabled" : "disabled"
    }

    const utf8Input = utf8Support === "enabled" || utf8Support === "input"

    let program: string | undefined = config.program
    if (!program) {
        program = window.activeTextEditor?.document.fileName
        if (!program) {
            program = await createHsptmp(utf8Input)
        }
    }

    config.program = program
    config.hsp3Root = config.hsp3Root || await selectHsp3Root()
    config.utf8Support = utf8Support
    config.distDir = config.distDir || distDir
    return config
}

export class MyConfigurationProvider implements DebugConfigurationProvider {
    public constructor(
        private readonly distDir: string,
    ) {
    }

    public resolveDebugConfiguration(
        _folder: WorkspaceFolder | undefined,
        config: DebugConfiguration,
        _token?: CancellationToken
    ): ProviderResult<DebugConfiguration> {
        return doResolveDebugConfiguration(config, this.distDir).catch(err => {
            const message = err instanceof Error ? err.message : String(err)
            window.showErrorMessage(message)
            return null
        })
    }
}
