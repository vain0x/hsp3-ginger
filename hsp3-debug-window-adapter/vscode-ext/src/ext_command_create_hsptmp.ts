import * as fsP from "fs/promises"
import * as path from "path"
import { encode } from "iconv-lite"
import { window, workspace } from "vscode"
import { selectHsp3Root } from "./ext_command_select_hsp3_root"
import { DomainError } from "./extension"

/**
 * 作業ディレクトリに hsptmp ファイルを生成する。
 */
export const createHsptmp = async (utf8Input?: boolean) => {
    const activeEditor = window.activeTextEditor
    if (!activeEditor) {
        throw new DomainError("エディターが開かれていません。")
    }

    // ファイルパスを構成する。
    let fileName = activeEditor.document.fileName
    const dirName = fileName
        ? path.dirname(fileName)
        : (
            workspace.workspaceFolders
            && workspace.workspaceFolders.length >= 1
            && workspace.workspaceFolders[0].uri.fsPath
            || await selectHsp3Root()
        )
    const hsptmpPath = path.join(dirName, "hsptmp")

    // ファイルを作成する。
    const text = activeEditor.document.getText()
    if (utf8Input !== false) {
        await fsP.writeFile(hsptmpPath, text)
    } else {
        // shift_jis (cp932) に変換してから出力する。
        await fsP.writeFile(hsptmpPath, encode(text, "cp932"), { encoding: "binary" })
    }

    return hsptmpPath
}
