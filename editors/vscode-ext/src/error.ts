export class CouldNotFindCompilerError extends Error {}

export class CouldNotExecuteError extends Error {}

export const errorGetMessage = (err: Error) => {
  if (err instanceof CouldNotFindCompilerError) {
    return "コンパイラのファイルパスを設定してください。"
  }

  if (err instanceof CouldNotExecuteError) {
    return `外部プログラムの起動に失敗しました: ${err.message}`
  }

  return "何らかのエラーが発生しました。"
}
