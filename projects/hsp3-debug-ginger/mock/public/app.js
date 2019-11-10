// クライアントサイド

const delay = timeout =>
  new Promise(resolve => setTimeout(resolve, timeout))

// ステータス欄を自動更新する。
const main = async () => {
  const statusElem = document.getElementById("status")
  const errorElem = document.getElementById("error")

  while (true) {
    const response = await fetch("/program/status")
    if (response.ok && response.status === 200) {
      const status = await response.json()
      statusElem.textContent = JSON.stringify(status, undefined, "  ")
    } else {
      const errorText = await response.text()
      errorElem.textContent = errorText
      console.error(errorText, response)
      await delay(3000)
    }
    await delay(100)
  }
}

main()
