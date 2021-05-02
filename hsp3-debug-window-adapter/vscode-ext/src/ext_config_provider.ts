import {
    CancellationToken,
    DebugConfiguration,
    DebugConfigurationProvider,
    ProviderResult,
    WorkspaceFolder,
    window,
} from "vscode"
import { selectHsp3Home } from "./ext_command_select_hsp3_home"
import { createHsptmp } from "./ext_command_create_hsptmp"
import { withNotify } from "./extension"
import { HSP3_LANG_ID } from "./ext_constants"

/**
 * デバッガーの設定を構成する。
 *
 * ここでの設定が "launch" リスエストに渡される。
 */
const doResolveDebugConfiguration = async (config: DebugConfiguration, outDir: string) => {
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

    config.program = config.program || await createHsptmp()
    config.hsp3Home = config.hsp3Home || await selectHsp3Home()
    config.outDir = config.outDir || outDir
    return config
}

export class MyConfigurationProvider implements DebugConfigurationProvider {
    public constructor(
        private readonly outDir: string,
    ) {
    }

    public resolveDebugConfiguration(
        _folder: WorkspaceFolder | undefined,
        config: DebugConfiguration,
        _token?: CancellationToken
    ): ProviderResult<DebugConfiguration> {
        return withNotify(async () => doResolveDebugConfiguration(config, this.outDir))()
    }
}
