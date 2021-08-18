const vscode = require("vscode")

const LANG_ID = "hsp3"

const activate = async context => {
  let currentLineComment = ";"

  // 設定を反映する。
  const onDidChangeConfiguration = () => {
    const config = vscode.workspace.getConfiguration("hsp3-vscode-syntax")
    const lineComment = config.get("line-comment")
    if (lineComment !== currentLineComment) {
      currentLineComment = lineComment
      context.subscriptions.push(vscode.languages.setLanguageConfiguration(LANG_ID, {
        comments: { lineComment },
      }))
    }
  }

  // 設定の変更を監視する。
  onDidChangeConfiguration()
  context.subscriptions.push(vscode.workspace.onDidChangeConfiguration(() => {
    onDidChangeConfiguration()
  }))
}

module.exports = {
  activate,
}
