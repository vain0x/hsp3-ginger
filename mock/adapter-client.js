// デバッグアダプターのクライアントとして動く websocket クライアント

const WS = require("websocket")

const WebSocketClient = WS.client

const WebSocketPort = 8089

const log = message => {
  const now = new Date().toISOString()
  console.log(`[${now}] ${message}`)
}

const start = ({ onConnected, onDisconnected, onReceived }) => {
  const client = new WebSocketClient()

  client.on("connectFailed", function (error) {
    log("接続 失敗 " + error.toString())
    onDisconnected()
  })

  client.on("connect", function (connection) {
    connection.on("error", function (error) {
      log("エラー " + error.toString())
    })

    connection.on("close", function () {
      log("切断")
      onDisconnected()
    })

    connection.on("message", function (message) {
      if (message.type !== "utf8") {
        log("受信 失敗 バイナリメッセージは受信できません")
        return
      }

      const payload = JSON.parse(message.utf8Data)
      log("受信 " + JSON.stringify(payload, undefined, 2))
      onReceived(payload)
    })

    const send = message => {
      if (!connection.connected) {
        log("送信 失敗 vscodeに接続していません")
        return
      }

      const payload = JSON.stringify(message, undefined, 2)
      log("送信 " + payload)
      connection.sendUTF(payload)
    }

    log("接続")
    onConnected({
      send,
    })
  })

  client.connect(`ws://localhost:${WebSocketPort}/`, "hsp3-debug-adapter-protocol")
}

module.exports = {
  start,
}
