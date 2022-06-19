import { ConfigurationTarget, Uri, window, workspace } from "vscode"
import { HSP3_CONFIG_SECTION, MY_CONFIG_SECTION } from "./ext_constants"
import { DomainError } from "./extension"

const HSP3_ROOT_KEY = "hsp3-root"

const doSelectHsp3Root = async () => {
    // この拡張機能の設定に指定されているものを使う。
    {
        const myConfig = workspace.getConfiguration(MY_CONFIG_SECTION)
        const hsp3Root = myConfig.get(HSP3_ROOT_KEY)
        if (typeof hsp3Root === "string" && hsp3Root !== "") {
            return hsp3Root
        }
    }

    // 言語の設定に定義されているものを使う。
    {
        const myConfig = workspace.getConfiguration(HSP3_CONFIG_SECTION)
        const hsp3Root = myConfig.get(HSP3_ROOT_KEY)
        if (typeof hsp3Root === "string" && hsp3Root !== "") {
            return hsp3Root
        }
    }

    // 環境変数に定義されているものを使う。
    {
        const hsp3Root = process.env.HSP3_ROOT
        if (typeof hsp3Root === "string" && hsp3Root !== "") {
            return hsp3Root
        }
    }

    // 選択してもらう。
    const paths = await window.showOpenDialog({
        canSelectFolders: true,
        defaultUri: Uri.parse("file:///C:/Program Files (x86)"),
        openLabel: "HSP3 のインストールディレクトリ",
    })

    const selectedPath = paths && paths[0] && paths[0].fsPath
    if (selectedPath) {
        // 選択結果をユーザー設定に保存する。
        {
            const myConfig = workspace.getConfiguration(MY_CONFIG_SECTION)
            await myConfig.update(HSP3_ROOT_KEY, selectedPath, ConfigurationTarget.Global)
        }
        return selectedPath
    }

    return null
}

export const selectHsp3Root = async () => {
    const hsp3Root = await doSelectHsp3Root()
    if (!hsp3Root) {
        throw new DomainError("HSP3 のインストールディレクトリが指定されていません。")
    }

    return hsp3Root
}
