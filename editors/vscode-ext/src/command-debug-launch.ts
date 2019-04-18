import * as vscode from "vscode"
import { configGetDebugCompilerOptions, configGetCompilerPath, configGetRoot } from "./config"

const TASK_SOURCE = "hsp3-ginger"
const TASK_KIND_TYPE = "hsp3-ginger"
const DEBUG_TASK_NAME = "debug"

/** デバッグ実行を開始するタスクを生成する。 */
export const createDebugTask = () => {
  const config = configGetRoot()
  const command = configGetCompilerPath(config)
  const args = configGetDebugCompilerOptions(config)

  const task = new vscode.Task(
    {
      type: TASK_KIND_TYPE,
    },
    vscode.TaskScope.Workspace,
    DEBUG_TASK_NAME,
    TASK_SOURCE,
    new vscode.ProcessExecution(command, args)
  )
  task.problemMatchers = []
  task.presentationOptions.reveal = vscode.TaskRevealKind.Silent
  return task
}

/** 上述のタスクを検索する。 */
const findDebugTask = async () => {
  const tasks = await vscode.tasks.fetchTasks()
    .then(tasks => tasks.filter(task => task.definition.type === TASK_KIND_TYPE && task.name === DEBUG_TASK_NAME))
  return tasks[0] || null
}

export const commandDebugLaunch = async () => {
  let task = await findDebugTask()
  if (!task) {
    // NOTE: タスクは自動生成されるので、必ず存在する。
    throw new Error("Missing debug task")
  }

  await vscode.tasks.executeTask(task)
}
