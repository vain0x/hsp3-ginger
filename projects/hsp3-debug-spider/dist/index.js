document.addEventListener("DOMContentLoaded", () => {
  const logmesElement = document.getElementById("logmes")

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
