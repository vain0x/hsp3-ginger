// Webサーバー

const express = require("express")
const app = express()
const path = require("path")
const connectWs = require("./adapter-client").start

const examplesDir = path.resolve(__dirname, "../examples")

const status = {
  starting: true,
  running: true,
  connected: false,
  line: 3,
  column: 1,
  fileName: path.join("examplesDir", "main.hsp"),
}

const WebServerPort = 8080

const sendWarn = message => {
  console.warn("VSCode に接続していないのでメッセージを送信できません。", message)
}

let send = sendWarn

const programRouter = () => {
  const router = express.Router()

  // 現在のステータスを返す。
  router.get("/status", (_request, response) => {
    return response.json(status)
  })

  // HSP のプログラム実行が開始されたかのように振る舞う。
  router.post("/start", (_request, response) => {
    status.starting = true
    status.running = true
    response.redirect("/")
  })

  // HSP のプログラム実行が終了した (end or ×ボタン) かのように振る舞う。
  router.post("/end", (_request, response) => {
    status.starting = false
    status.running = false
    send({ type: "end" })
    return response.redirect("/")
  })

  // HSP プログラムの実行中に assert が失敗したかのように振る舞う。
  router.post("/assert", (_request, response) => {
    status.running = false
    send({
      type: "stopOnBreakpoint",
      ...status,
    })
    return response.redirect("/")
  })
  return router
}

const connectLoop = async () => {
  while (true) {
    await new Promise(resolve => {
      connectWs({
        onConnected(props) {
          status.connected = true
          status.running = true
          send = props.send
        },
        onDisconnected() {
          status.connected = false
          send = sendWarn
          resolve()
        },
        onReceived(message) {
          switch (message.event) {
            case "continue": {
              console.log("操作 再開")
              status.running = true
              break
            }
            case "pause": {
              console.log("操作 中断")
              status.running = false
              break
            }
            case "step":
              console.log("操作 次へ")
              status.line += 1
              send({
                type: "stopOnBreakpoint",
                line: status.line,
              })
              break
            default:
              console.log(`受信 不明なメッセージ ${JSON.stringify(message)}`)
              break
          }
        },
      })
    }).catch(err => {
      console.error(err)
    })

    await new Promise(resolve => setTimeout(resolve, 1000))
  }
}

const main = () => {
  connectLoop()

  app.use("/program", programRouter())

  // ./public 以下のファイルを公開する。
  app.use(express.static("public"))

  app.listen(WebServerPort, () => {
    console.log(`Listening http://localhost:${WebServerPort}`)
  })
}

main()
