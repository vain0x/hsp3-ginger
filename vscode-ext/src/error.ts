export class CouldNotExecuteError extends Error {}

export const errorGetMessage = (err: Error) => {
  if (err instanceof CouldNotExecuteError) {
    return `外部プログラムの起動に失敗しました: ${err.message}`
  }

  return "何らかのエラーが発生しました。"
}
