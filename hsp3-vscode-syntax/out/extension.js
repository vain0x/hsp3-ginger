const vscode = require("vscode")

const activate = () => {}

vscode.languages.registerColorProvider({
  language: "hsp3",
}, {
  provideColorPresentations: (color, context, _ct) => {
    const f = n => Math.round(n * 255)
    const label = `color ${f(color.red)}, ${f(color.green)}, ${f(color.blue)}`
    return [
      {
        label,
        textEdit: {
          range: context.range,
          newText: label,
        },
      },
    ]
  },
  provideDocumentColors: async (document, _ct) => {
    const colorInfoArray = []

    for (const { line, row } of document.getText().split(/\r?\n/g).map((line, row) => ({ line, row }))) {
      let keyword = line.indexOf("color")
      let t = 1
      let i = keyword + "color".length
      if (keyword < 0) {
        keyword = line.indexOf("hsvcolor")
        t = 2
        i = keyword + "hsvcolor".length
      }
      if (keyword < 0 || line.substring(0, keyword).trim() !== "") {
        continue
      }

      let ok = true
      const params = line.substring(i).split(",").map(param => {
        param = param.trim()
        if (param === "") {
          return 0
        }

        const value = Number.parseInt(param)
        ok = ok && !Number.isNaN(value)
        return value
      })

      if (!ok) {
        continue
      }

      const [red, green, blue] = params

      colorInfoArray.push({
        range: {
          start: {
            line: row,
            character: keyword,
          },
          end: {
            line: row,
            character: line.length,
          },
        },
        color: { red: red / 255, green: green / 255, blue: blue / 255, alpha: 1 },
      })
    }

    return colorInfoArray
  },
})

module.exports = {
  activate,
}
