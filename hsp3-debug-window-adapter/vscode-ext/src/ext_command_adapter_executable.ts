import  * as fs from "fs"
import * as path from "path"
import { window } from "vscode"

const fileExists = (fileName: string) =>
    new Promise<boolean>(resolve =>
        fs.stat(fileName, err => resolve(!err)))

/**
 * デバッガーアダプターの起動コマンドを構成する。
 */
export const adapterExecutable = (extensionRoot: string) => async () => {
    const program = path.join(extensionRoot, "dap_index.js")

    if (!await fileExists(program)) {
        window.showErrorMessage("ファイルが見つかりません: " + program)
    }

    return {
        command: "node",
        args: [
            program,
        ],
    }
}
