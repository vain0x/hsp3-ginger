import * as path from "path"

/**
 * デバッガーアダプターの起動コマンドを構成する。
 */
export const adapterExecutableCommand = (extensionRoot: string) => async () => {
    const program = path.join(extensionRoot, "dap_index.js")

    return {
        command: "node",
        args: [
            program,
        ],
    }
}
