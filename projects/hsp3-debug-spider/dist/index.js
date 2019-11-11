document.addEventListener("DOMContentLoaded", () => {
  const continueButtonElement = document.getElementById("continue-button")
  const logmesElement = document.getElementById("logmes")

  continueButtonElement.addEventListener("click", () => {
    fetch("/continue", { method: "POST" })
      .catch(err => console.error(err))
  })

  const pollLogmes = () => {
    setTimeout(() => {
      fetch("/logmes", { method: "GET" })
        .then(res => res.text())
        .then(logmes => logmesElement.textContent = logmes)
        .then(() => pollLogmes())
        .catch(err => console.error(err))
    }, 300)
  }

  pollLogmes()
})
