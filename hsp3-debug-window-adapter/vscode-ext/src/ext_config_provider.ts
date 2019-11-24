import {
    CancellationToken,
    DebugConfiguration,
    DebugConfigurationProvider,
    ProviderResult,
    WorkspaceFolder,
} from "vscode"
import { selectHsp3Root } from "./ext_command_select_hsp3_root"
import { createHsptmp } from "./ext_command_create_hsptmp"
import { withNotify } from "./extension"

export class MyConfigurationProvider implements DebugConfigurationProvider {
    public constructor(
        private readonly _extensionRoot: string,
    ) {
    }

    public resolveDebugConfiguration(
        _folder: WorkspaceFolder | undefined,
        config: DebugConfiguration,
        _token?: CancellationToken
    ): ProviderResult<DebugConfiguration> {
        // デバッガーの設定を構成する。
        // ここでの設定が "launch" リスエストに渡される。
        return withNotify(async () => {
            config.program = config.program || await createHsptmp()
            config.hsp3Root = config.hsp3Root || await selectHsp3Root()
            config.extensionRoot = config.extensionRoot || this._extensionRoot
            return config
        })()
    }
}
